use crate::{
    local_repository_path, GetRepositoryId, OwnedRepository, PartialRepositoryId, RepositoryId,
    Secret, StarredRepository,
};
use anyhow::{bail, Context, Error, Result};
use async_stream::try_stream;
use async_trait::async_trait;
use dialoguer::Confirm;
use futures::{future, stream::TryStreamExt, Stream};
use git2::{
    build::RepoBuilder, Branch, Cred, FetchOptions, IndexAddOption, PushOptions, RemoteCallbacks,
    Repository as GitRepository,
};
use http::header::HeaderName;
use indoc::formatdoc;
use octocrab::{models::Repository as GitHubRepository, Octocrab, Page};
use serde::{Deserialize, Serialize};
use std::{env, fmt, future::Future, path::Path, process::Command};

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct AppConfig<'a> {
    pub github_username: &'a str,
    pub github_token: &'a str,
    pub workspace_root_dir: &'a Path,
}

#[derive(Debug)]
pub struct App<'a> {
    github_username: &'a str,
    github_token: Secret<&'a str>,
    workspace_root_dir: &'a Path,
}

impl<'a> App<'a> {
    pub fn new(
        AppConfig { github_username, github_token, workspace_root_dir }: AppConfig<'a>,
    ) -> Result<Self, Error> {
        let github_token = github_token.into();
        let s = Self { github_username, github_token, workspace_root_dir };
        Ok(s)
    }

    pub async fn view_repository_settings(
        &self,
        repo_id: PartialRepositoryId,
    ) -> Result<(), Error> {
        let RepositoryId { owner, name } = repo_id.complete(self.github_username);

        let client = create_client()?;
        let repo = client.repos(owner, name).get().await?;
        let settings = repo.extract_repository_settings()?;

        println!("{}", settings);

        Ok(())
    }

    pub async fn apply_repository_settings(
        &self,
        from: PartialRepositoryId,
        to: PartialRepositoryId,
    ) -> Result<(), Error> {
        let from = from.complete(self.github_username);
        let to = to.complete(self.github_username);

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

        let old_settings = get_settings(to.clone()).await?;
        let new_settings = get_settings(from.clone()).await?;
        let diff = RepositorySettingsDiff::new(&old_settings, &new_settings);

        println!("{}", diff);

        if !Confirm::new()
            .with_prompt("Apply settings?")
            .default(false)
            .show_default(true)
            .wait_for_newline(true)
            .interact()?
        {
            return Ok(());
        }

        let _: GitHubRepository = {
            let RepositoryId { owner, name } = to;
            client.patch(format!("repos/{owner}/{name}"), Some(&new_settings)).await?
        };

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
            .try_for_each(|repo| {
                println!("{}", repo);
                future::ok(())
            })
            .await?;
        Ok(())
    }

    pub async fn open_repository(
        &self,
        repo_id: PartialRepositoryId,
        upstream: bool,
    ) -> Result<(), Error> {
        let RepositoryId { owner, name } = repo_id.complete(self.github_username);

        let client = create_client()?;

        let repo = client.repos(&owner, &name).get().await;
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
                    .sort("pushed")
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
            .and_then(|repo| async {
                let repo_id = repo.get_repository_id()?;
                let client = create_client()?;
                let commits =
                    client.repos(repo_id.owner, repo_id.name).commits().per_page(1).send().await?;
                let commit = commits.items.first().map(ToOwned::to_owned);
                Ok(OwnedRepository(repo, commit))
            })
            .try_for_each(|repo| {
                println!("{}", repo);
                future::ok(())
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
        let repo_id = repo_id.complete(self.github_username);

        if repo_id.owner != self.github_username {
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
        let path = local_repository_path(workspace_home, &repo_id);
        println!("Cloning {repo_id} repository to {path}.", path = path.display());
        let repo = RepoBuilder::new()
            .fetch_options(create_fetch_options())
            .clone(&ssh_url, &path)
            .context("Failed to clone repository.")?;

        if let Some(upstream_url) = upstream_url {
            let mut remote =
                repo.remote("upstream", &upstream_url).context("Failed to add upstream remote.")?;
            let mut options = {
                let mut opts = create_fetch_options();
                opts.prune(git2::FetchPrune::On);
                opts
            };
            remote
                .fetch(&["+refs/heads/*:refs/remotes/origin/*"], Some(&mut options), None)
                .context("Failed to fetch upstream.")?;
        }

        Ok(())
    }

    pub async fn dump_changes(&self) -> Result<(), Error> {
        let repo = GitRepository::discover("./")?;

        let head = repo.head()?;
        if !head.is_branch() {
            bail!("HEAD is not a branch.")
        }
        let local_branch = Branch::wrap(head);
        match local_branch.name()? {
            Some(name) => {
                if name != "master" {
                    bail!("Can only dump `master` branch.")
                }
            }
            None => bail!("Branch name is not a valid utf-8."),
        };
        let mut remote = repo.find_remote("origin")?;

        // Add.
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;

        // Commit.
        let signature = repo.signature()?;
        let tree = repo.find_tree(index.write_tree()?)?;
        let parent = local_branch.into_reference().peel_to_commit()?;
        repo.commit("HEAD".into(), &signature, &signature, "dump (Shub)", &tree, &[&parent])?;

        // Push.
        let refspecs = vec!["refs/heads/master:refs/heads/master"];
        let mut opts = create_push_options();
        remote.push(&refspecs, (&mut opts).into())?;

        Ok(())
    }
}

fn create_fetch_options<'a>() -> FetchOptions<'a> {
    let mut opts = FetchOptions::new();
    opts.remote_callbacks(create_remote_callbacks());
    opts
}

fn create_push_options<'a>() -> PushOptions<'a> {
    let mut opts = PushOptions::new();
    opts.remote_callbacks(create_remote_callbacks());
    opts
}

fn create_remote_callbacks<'a>() -> RemoteCallbacks<'a> {
    let mut cbs = RemoteCallbacks::new();
    cbs.credentials(|_url, username_from_url, _credential_type| {
        let username = username_from_url.unwrap_or("git");
        Cred::ssh_key_from_agent(username)
    });
    cbs
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

macro_rules! write_key {
    ($w:expr, $this:expr, $key:ident) => {{
        let val = $this.$key;
        write!($w, "{key:>25} = {val:5}\n", key = stringify!($key))
    }};
}

impl fmt::Display for RepositorySettings {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_key!(f, self, allow_rebase_merge)?;
        write_key!(f, self, allow_squash_merge)?;
        write_key!(f, self, allow_auto_merge)?;
        write_key!(f, self, delete_branch_on_merge)?;
        write_key!(f, self, allow_merge_commit)?;
        Ok(())
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
struct RepositorySettingsDiff<'a> {
    old: &'a RepositorySettings,
    new: &'a RepositorySettings,
}

impl<'a> RepositorySettingsDiff<'a> {
    fn new(old: &'a RepositorySettings, new: &'a RepositorySettings) -> Self {
        Self { old, new }
    }
}

macro_rules! diff_key {
    ($w:expr, $this:expr, $key:ident) => {{
        let old = $this.old.$key;
        let new = $this.new.$key;
        write!($w, "{key:>25} = {old:>5} -> {new:<5}\n", key = stringify!($key))
    }};
}

impl fmt::Display for RepositorySettingsDiff<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        diff_key!(f, self, allow_rebase_merge)?;
        diff_key!(f, self, allow_squash_merge)?;
        diff_key!(f, self, allow_auto_merge)?;
        diff_key!(f, self, delete_branch_on_merge)?;
        diff_key!(f, self, allow_merge_commit)?;
        Ok(())
    }
}

fn create_client() -> Result<Octocrab, Error> {
    let user_agent =
        concat!(env!("CARGO_PKG_NAME"), concat!("/", env!("CARGO_PKG_VERSION"))).to_owned();
    let token = env::var("SHUB_TOKEN")?;
    let client = Octocrab::builder()
        .add_header(HeaderName::from_static("user-agent"), user_agent)
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
