use crate::cli::LangFilter;
use anyhow::Result;
use futures::{future, stream::TryStreamExt};
use serde::{Deserialize, Serialize};
use shub::github::{
    client::GhClient,
    requests::{RepositoryType, UpdateRepository},
    responses::{MyRepository, Repository, StarredRepository, WorkflowRun},
};
use std::{borrow::Cow, fmt, io::Write, path::Path};
use tabwriter::TabWriter;
use tokio::fs;
use tracing::debug;

#[derive(Debug)]
pub struct App<'a> {
    pub username: &'a str,
    pub client: GhClient,
}

impl App<'_> {
    pub async fn delete_all_workflow_runs(&self, owner: Option<&str>, repo: &str) -> Result<()> {
        let owner = owner.unwrap_or(self.username);
        println!("Deleting workflow runs in {}/{}.", owner, repo);
        let deleted = self
            .client
            .actions()
            .list_workflow_runs(owner, repo)
            .and_then(move |WorkflowRun { id, .. }| async move {
                self.client.actions().delete_workflow_run(owner, repo, id).await?;
                Ok(())
            })
            .try_fold(0, |acc, _| future::ok(acc + 1))
            .await?;
        println!("{} workflow runs deleted.", deleted);
        Ok(())
    }

    pub async fn download_settings(
        &self,
        owner: Option<&str>,
        repo: &str,
        file: &Path,
    ) -> Result<()> {
        let owner = owner.unwrap_or(self.username);
        let path = file;
        println!("Downloading GitHub repository settings for {}/{} to {:?}.", owner, repo, path);
        let settings: RepositorySettings =
            self.client.repos().get_repository(owner, repo).await?.into();
        let buf = toml::to_vec(&settings)?;
        debug!(?settings, ?path, "writing settings");
        fs::write(path, &buf).await?;
        Ok(())
    }

    pub async fn apply_settings(&self, owner: Option<&str>, repo: &str, file: &Path) -> Result<()> {
        let owner = owner.unwrap_or(self.username);
        let path = file;
        println!("Applying GitHub repository settings from {:?} for {}/{}.", path, owner, repo,);
        let settings = fs::read(path).await?;
        let settings: RepositorySettings = toml::from_slice(&settings)?;
        debug!(?settings, "applying settings");
        let settings = settings.into();
        self.client.repos().update_repository(owner, repo, &settings).await?;
        Ok(())
    }

    pub async fn list_starred(&self, lang_filter: Option<&LangFilter>, short: bool) -> Result<()> {
        let mut out = {
            let w = std::io::stdout();
            TabWriter::new(w)
        };

        let starred = self
            .client
            .activity()
            .get_starred()
            .try_filter(|repo| {
                let pass = lang_filter
                    .and_then(|LangFilter { negation, lang }| {
                        repo.language
                            .as_ref()
                            .map(|x| x.to_ascii_lowercase() == lang.to_ascii_lowercase())
                            .or_else(|| false.into())
                            .map(|x| if *negation { !x } else { x })
                    })
                    .unwrap_or(true);
                future::ready(pass)
            })
            .try_collect::<Vec<_>>()
            .await?;

        let starred = Tabulator { repos: starred, short };
        write!(&mut out, "{}", starred)?;
        out.flush()?;

        Ok(())
    }

    pub async fn list_repos(&self) -> Result<()> {
        let mut out = {
            let w = std::io::stdout();
            TabWriter::new(w)
        };

        let client = self.client.repos();
        let repos = client.list_my_repositories(RepositoryType::Owner.into());
        let repos: Vec<_> = repos.try_collect().await?;

        let tabulator = Tabulator { repos, short: false };
        write!(&mut out, "{}", tabulator)?;
        out.flush()?;

        Ok(())
    }
}

#[derive(Debug)]
struct Tabulator<R> {
    repos: Vec<R>,
    short: bool,
}

impl fmt::Display for Tabulator<StarredRepository> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ellipsize: fn(_, _) -> _ = if self.short {
            ellipsize
        } else {
            // noop
            (|x, _| Cow::Borrowed(x)) as _
        };

        for repo in &self.repos {
            // print name
            let name = &repo.full_name;
            let name = ellipsize(name, 40);
            write!(f, "{}", name)?;

            // print description
            write!(f, "\t",)?;
            let desc = repo.description.as_ref().map(String::as_str).unwrap_or("");
            let desc = ellipsize(desc, 80);
            write!(f, "{}", desc)?;

            // print language
            write!(f, "\t",)?;
            let lang = repo.language.as_ref().map(String::as_str).unwrap_or("");
            let lang = ellipsize(lang, 20);
            write!(f, "{}", lang)?;

            write!(f, "\n")?;
        }

        Ok(())
    }
}

impl fmt::Display for Tabulator<MyRepository> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // _monkey patch_ ellipsize
        let ellipsize: fn(_, _) -> _ = if self.short {
            ellipsize
        } else {
            // noop
            (|x, _| Cow::Borrowed(x)) as _
        };

        for repo in &self.repos {
            // print name
            let name = &repo.full_name;
            let name = ellipsize(name, 40);
            write!(f, "{}", name)?;

            // print description
            write!(f, "\t",)?;
            let desc = repo.description.as_ref().map(String::as_str).unwrap_or("");
            let desc = ellipsize(desc, 80);
            write!(f, "{}", desc)?;

            // print archived status
            write!(f, "\t",)?;
            let archive = if repo.archived { "archived" } else { "" };
            write!(f, "{}", archive)?;

            // print visiblity
            write!(f, "\t",)?;
            let visibility = {
                use shub::github::responses::RepositoryVisibility::*;
                match repo.visibility {
                    Public => "public",
                    Private => "private",
                }
            };
            write!(f, "{}", visibility)?;

            // print language
            write!(f, "\t",)?;
            let lang = repo.language.as_ref().map(String::as_str).unwrap_or("");
            let lang = ellipsize(lang, 20);
            write!(f, "{}", lang)?;

            write!(f, "\n")?;
        }

        Ok(())
    }
}

fn ellipsize(text: &str, threshold: usize) -> Cow<'_, str> {
    // todo(kfj): convert to type error?
    debug_assert!(threshold > 3);

    if text.len() <= threshold {
        text.into()
    } else {
        let text = text.chars().take(threshold - 3);
        let ellipsis = (0..3).map(|_| '.');
        let s: String = text.chain(ellipsis).collect();
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

#[derive(Deserialize, Serialize, Debug)]
struct RepositorySettings {
    allow_rebase_merge: bool,
    allow_squash_merge: bool,
    allow_auto_merge: bool,
    delete_branch_on_merge: bool,
    allow_merge_commit: bool,
}

impl From<Repository> for RepositorySettings {
    fn from(
        Repository {
            allow_rebase_merge,
            allow_squash_merge,
            allow_auto_merge,
            delete_branch_on_merge,
            allow_merge_commit,
            ..
        }: Repository,
    ) -> Self {
        Self {
            allow_rebase_merge,
            allow_squash_merge,
            allow_auto_merge,
            delete_branch_on_merge,
            allow_merge_commit,
        }
    }
}

impl Into<UpdateRepository> for RepositorySettings {
    fn into(self) -> UpdateRepository {
        let RepositorySettings {
            allow_squash_merge,
            allow_merge_commit,
            allow_rebase_merge,
            allow_auto_merge,
            delete_branch_on_merge,
        } = self;
        let allow_squash_merge = allow_squash_merge.into();
        let allow_merge_commit = allow_merge_commit.into();
        let allow_rebase_merge = allow_rebase_merge.into();
        let allow_auto_merge = allow_auto_merge.into();
        let delete_branch_on_merge = delete_branch_on_merge.into();
        UpdateRepository {
            allow_squash_merge,
            allow_merge_commit,
            allow_rebase_merge,
            allow_auto_merge,
            delete_branch_on_merge,
        }
    }
}
