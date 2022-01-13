use crate::{PartialRepositoryId, RepositoryId};
use anyhow::{bail, Error, Result};
use async_stream::try_stream;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use dialoguer::Confirm;
use futures::{future, stream::TryStreamExt, Stream, StreamExt};
use git2::Repository as GitRepository;
use http::{header::HeaderName, StatusCode};
use indoc::formatdoc;
use octocrab::{models::Repository as GitHubRepository, Octocrab, Page};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    env, fmt,
    future::Future,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use tokio::fs;
use tracing::debug;

macro_rules! write_col {
    ($w:expr, $len:expr, $txt:expr) => {
        write!($w, "{:len$}", ellipsize($txt, $len as _), len = $len as _)
    };
    (, $w:expr, $len:expr, $txt:expr) => {
        write!($w, " | {:len$}", ellipsize($txt, $len as _), len = $len as _)
    };
}

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), concat!("/", env!("CARGO_PKG_VERSION")));

#[derive(Debug)]
pub struct App<'a> {
    pub username: &'a str,
}

impl App<'_> {
    pub async fn get_repository_settings(&self, repo_id: PartialRepositoryId) -> Result<(), Error> {
        let RepositoryId { owner, name } = repo_id.complete(self.username);

        let client = create_client()?;
        let repo = client.repos(owner, name).get().await?;
        let settings = repo.extract_repository_settings()?;

        println!("{:#?}", settings);

        Ok(())
    }

    pub async fn apply_repository_settings(
        &self,
        from: PartialRepositoryId,
        to: PartialRepositoryId,
    ) -> Result<(), Error> {
        let from = from.complete(self.username);
        let to = to.complete(self.username);

        let client = create_client()?;

        let get_settings = |repo_id: RepositoryId| {
            let client = client.clone();
            let RepositoryId { owner, name } = repo_id;
            let owner = owner.to_owned();
            let name = name.to_owned();
            async move {
                let repo = client.repos(owner, name).get().await?;
                let settings = repo.extract_repository_settings()?;
                Result::<_, Error>::Ok(settings)
            }
        };

        // Get repository settings.
        let old_settings = get_settings(to.clone()).await?;
        let new_settings = get_settings(from.clone()).await?;

        println!("{:#?}", new_settings);

        if !Confirm::new()
            .with_prompt("Apply settings?")
            .default(false)
            .show_default(true)
            .wait_for_newline(true)
            .interact()?
        {
            return Ok(());
        }

        // Apply settings.
        let RepositoryId { owner, name } = to;
        client
            .patch(format!("repos/{owner}/{name}", owner = owner, name = name), Some(&new_settings))
            .await?;

        Ok(())
    }

    pub async fn list_starred_repositories(&self) -> Result<(), Error> {
        let repos = unpage(&|page_num| async move {
            let client = create_client()?;
            let req = {
                let b = client
                    .current()
                    .list_repos_starred_by_authenticated_user()
                    .sort("updated")
                    .per_page(100);
                let b = match page_num {
                    Some(x) => b.page(x),
                    None => b,
                };
                b
            };
            let repos = req.send().await?;
            Ok(repos)
        });
        repos
            .map_ok(StarredRepository)
            .try_for_each(|repo| async move {
                println!("{}", repo);
                Ok(())
            })
            .await?;
        Ok(())
    }

    pub async fn open_repository(
        &self,
        owner: Option<&str>,
        name: &str,
        upstream: bool,
    ) -> Result<(), Error> {
        let owner = owner.unwrap_or(self.username);
        let client = create_client()?;

        let repo = client.repos(owner, name).get().await;
        let repo = match repo {
            Ok(x) => x,
            Err(err) => {
                if matches!(&err, octocrab::Error::GitHub { source, .. } if source.message == "Not Found")
                {
                    bail!("Repository {}/{} does not exist.", owner, name)
                } else {
                    return Err(err.into());
                }
            }
        };

        {
            let s = repo.extract_repository_settings();
            println!("{:?}", s);
        }

        let url = if upstream {
            if !repo.fork.unwrap_or_default() {
                bail!("Repository {}/{} is not a fork.", owner, name)
            }
            repo.parent
                .map(|x| x.html_url)
                .flatten()
                .expect("Forked repository should have the HTML URL to its parent repository.")
        } else {
            repo.html_url.expect("Repository should have the HTML URL to itself.")
        };

        Command::new("xdg-open").arg(url.as_str()).status()?;
        Ok(())
    }

    pub async fn list_owned_repositories(&self) -> Result<(), Error> {
        let repos = unpage(&|page_num| async move {
            let client = create_client()?;
            let req = {
                let b = client
                    .current()
                    .list_repos_for_authenticated_user()
                    .type_("owner")
                    .sort("updated")
                    .per_page(100);
                let b = match page_num {
                    Some(x) => b.page(x),
                    None => b,
                };
                b
            };
            let repos = req.send().await?;
            Ok(repos)
        });
        repos
            .map_ok(OwnedRepository)
            .try_for_each(|repo| async move {
                println!("{}", repo);
                Ok(())
            })
            .await?;
        Ok(())
    }

    pub async fn fork_repository(&self, repo_id: RepositoryId) -> Result<(), Error> {
        let client = create_client()?;
        client.repos(&repo_id.owner, &repo_id.name).create_fork().send().await?;
        Ok(())
    }

    pub async fn clone_repository(&self, repo_id: PartialRepositoryId) -> Result<(), Error> {
        let repo_id = repo_id.complete(self.username);

        if repo_id.owner != self.username {
            panic!();
        }

        let client = create_client()?;
        let repo_info = client.repos(&repo_id.owner, &repo_id.name).get().await?;

        let ssh_url = repo_info
            .ssh_url
            .ok_or_else(|| Error::msg("Expecting repository to have ssh url, but was not."))?;

        let upstream_url = match repo_info.parent {
            Some(upstream) => upstream
                .ssh_url
                .ok_or_else(|| {
                    Error::msg("Expecting upstream repository to have ssh url, but was not.")
                })?
                .into(),
            None => None,
        };

        let workspace_home = env::var("WORKSPACE_HOME")?;
        let path = local_repository_path(workspace_home, repo_id);
        println!("Cloning repository to {}.", path.display());
        let repo = GitRepository::clone(&ssh_url, path)?;

        if let Some(upstream_url) = upstream_url {
            println!("Adding a remote for `upstream` at `{}`.", upstream_url);
            repo.remote("upstream", &upstream_url)?;
        }

        Ok(())
    }
}

fn ellipsize(text: &str, threshold: usize) -> Cow<'_, str> {
    debug_assert!(threshold > 3);
    if text.len() <= threshold {
        text.into()
    } else {
        let text = text.chars().take(threshold - 3);
        let s: String = text.chain("...".chars()).collect();
        s.into()
    }
}

#[cfg(test)]
#[test]
fn test_ellipsize() {
    use quickcheck::{quickcheck, TestResult};

    fn has_max_length_threshold(text: String, threshold: usize) -> TestResult {
        if threshold < 4 {
            return TestResult::discard();
        }
        TestResult::from_bool(ellipsize(&text, threshold).chars().count() <= threshold)
    }

    quickcheck(has_max_length_threshold as fn(_, _) -> TestResult);

    fn has_ellipsis_at_the_end(text: String, threshold: usize) -> TestResult {
        if threshold < 4 {
            return TestResult::discard();
        }
        if text.chars().count() <= threshold {
            return TestResult::discard();
        }
        let ellipsized = ellipsize(&text, threshold);
        TestResult::from_bool(ellipsized.ends_with("..."))
    }

    quickcheck(has_ellipsis_at_the_end as fn(_, _) -> TestResult);
}

trait ExtractRepositorySettings {
    fn extract_repository_settings(&self) -> Result<RepositorySettings, Error>;
}

#[derive(Deserialize, Serialize, PartialEq, Copy, Clone, Debug)]
struct RepositorySettings {
    allow_rebase_merge: bool,
    allow_squash_merge: bool,
    allow_auto_merge: bool,
    delete_branch_on_merge: bool,
    allow_merge_commit: bool,
}

impl RepositorySettings {
    async fn read_from(path: &Path) -> Result<Self, Error> {
        let buf = fs::read(path).await?;
        let s = toml::from_slice(&buf)?;
        Ok(s)
    }
}

macro_rules! extract_key {
    ($repo:expr, $key:ident) => {
        $repo.$key.ok_or_else(|| {
            Error::msg(formatdoc!("Missing value for key `{key}`.", key = stringify!($key)))
        })
    };
}

impl ExtractRepositorySettings for GitHubRepository {
    fn extract_repository_settings(&self) -> Result<RepositorySettings, Error> {
        let repo = self;
        let s = RepositorySettings {
            allow_rebase_merge: extract_key!(repo, allow_rebase_merge)?,
            allow_squash_merge: extract_key!(repo, allow_squash_merge)?,
            allow_auto_merge: extract_key!(repo, allow_auto_merge)?,
            delete_branch_on_merge: extract_key!(repo, delete_branch_on_merge)?,
            allow_merge_commit: extract_key!(repo, allow_merge_commit)?,
        };
        Ok(s)
    }
}

#[derive(PartialEq, Clone, Debug)]
struct RepoSettingsDiff {
    old: RepositorySettings,
    new: RepositorySettings,
}

macro_rules! diff_key {
    ($w:expr, $this:expr, $key:ident) => {{
        let old = $this.old.$key;
        let new = $this.new.$key;
        write!($w, "{key} | {old} -> {new}\n", key = stringify!(key), old = old, new = new)
    }};
}

impl fmt::Display for RepoSettingsDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        diff_key!(f, self, allow_rebase_merge)?;
        diff_key!(f, self, allow_squash_merge)?;
        diff_key!(f, self, allow_auto_merge)?;
        diff_key!(f, self, delete_branch_on_merge)?;
        diff_key!(f, self, allow_merge_commit)?;
        Ok(())
    }
}

#[derive(PartialEq, Debug)]
pub struct LanguageFilter {
    pub negation: bool,
    pub language: String,
}

fn create_client() -> Result<Octocrab, Error> {
    let token = env::var("SHUB_TOKEN").expect("GITHUB_TOKEN env variable is required");
    let client = Octocrab::builder()
        .add_header(HeaderName::from_static("user-agent"), USER_AGENT.to_owned())
        .personal_token(token)
        .build()?;
    Ok(client)
}

fn unpage<'a, T, F>(
    factory: &'a dyn Fn(Option<u8>) -> F,
) -> impl Stream<Item = Result<T, Error>> + 'a
where
    T: 'a + Send,
    F: Future<Output = Result<Page<T>, Error>>,
{
    try_stream! {
        let mut page_num = None;
        loop {
            let req = (factory)(page_num);
            let page = req.await?;
            let has_next = page.next.is_some();
            for repo in page {
                yield repo;
            }
            if !has_next {
                break;
            }
            page_num = (page_num.unwrap_or(1) + 1).into();
        }
    }
}

const REPO_NAME_LEN: u8 = 30;
const OWNER_NAME_LEN: u8 = 20;

#[derive(PartialEq, Clone, Debug)]
struct StarredRepository(GitHubRepository);

impl fmt::Display for StarredRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repo = &self.0;

        let name = &repo.name;
        write_col!(f, REPO_NAME_LEN, name)?;

        let desc = repo.description.as_ref().map(|x| x.as_str()).unwrap_or_default();
        write_col!(, f, 60, desc)?;

        let owner = repo.owner.as_ref().map(|x| x.login.as_str()).unwrap_or_default();
        write_col!(, f, OWNER_NAME_LEN, owner)?;

        let lang = repo.language.as_ref().map(|x| x.as_str()).flatten().unwrap_or_default();
        write_col!(, f, 10, lang)?;

        Ok(())
    }
}

#[derive(PartialEq, Clone, Debug)]
struct OwnedRepository(GitHubRepository);

impl fmt::Display for OwnedRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repo = &self.0;

        let visibility =
            repo.private.map(|x| if x { "private" } else { "public" }).unwrap_or_default();
        write_col!(f, 7, visibility)?;

        let name = &repo.name;
        write_col!(, f, REPO_NAME_LEN, name)?;

        let desc = repo.description.as_ref().map(|x| x.as_str()).unwrap_or_default();
        write_col!(, f, 60, desc)?;

        let updated = repo
            .pushed_at
            .as_ref()
            .map(|x| x.relative_from_now())
            .map(|x| Cow::Owned(x))
            .unwrap_or_default();
        write_col!(, f, 15, &updated)?;

        let lang = repo.language.as_ref().map(|x| x.as_str()).flatten().unwrap_or_default();
        write_col!(, f, 10, lang)?;

        let archived = repo.archived.map(|x| if x { "archived" } else { "" }).unwrap_or_default();
        write_col!(, f, 8, archived)?;

        Ok(())
    }
}

/// Relative time from now.
trait RelativeFromNow {
    fn relative_from_now(&self) -> String;
}

impl<T> RelativeFromNow for DateTime<T>
where
    T: TimeZone,
{
    fn relative_from_now(&self) -> String {
        let duration = Utc::now().signed_duration_since(self.clone());
        let age = Since(duration);
        age.to_string()
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
struct Since(chrono::Duration);

impl fmt::Display for Since {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let days = self.0.num_days();
        match days {
            _ if days < 1 => {
                write!(f, "today")
            }
            _ if days < 7 => {
                write!(f, "this week")
            }
            _ if days < 30 => {
                write!(f, "this month")
            }
            _ if days < 365 => {
                write!(f, "this year")
            }
            _ => {
                let years = days / 365;
                if years == 1 {
                    write!(f, "{} year ago", years)
                } else {
                    write!(f, "{} years ago", years)
                }
            }
        }
    }
}

fn local_repository_path(workspace: impl AsRef<Path>, repo_id: RepositoryId) -> PathBuf {
    workspace.as_ref().to_path_buf().join(repo_id.owner).join(repo_id.name)
}

#[cfg(test)]
#[test]
fn test_local_repository_path() {
    let workspace = "./workspace";
    let path = local_repository_path(workspace, RepositoryId::new("kafji", "shub"));
    assert_eq!(path.display().to_string(), "./workspace/kafji/shub");
}
