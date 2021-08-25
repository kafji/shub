use crate::cli::LangFilter;
use anyhow::Result;
use futures::{future, stream::TryStreamExt};
use serde::{Deserialize, Serialize};
use shub::{
    client::GhClient,
    requests::UpdateRepository,
    responses::{Repository, StarredRepository, WorkflowRun},
};
use std::{fmt, io::Write, path::Path};
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

    pub async fn list_starred(&self, lang_filter: Option<&LangFilter>) -> Result<()> {
        let out = std::io::stdout();
        let mut out = TabWriter::new(out);

        let starred = self
            .client
            .activity()
            .get_starred()
            .try_filter(|repo| {
                let pass = lang_filter
                    .and_then(|filter| {
                        let LangFilter { negation, lang } = filter;
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
        let starred = StarredRepositories(starred);
        write!(&mut out, "{}", starred)?;
        out.flush()?;

        Ok(())
    }
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

#[derive(Debug)]
struct StarredRepositories(Vec<StarredRepository>);

impl fmt::Display for StarredRepositories {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for x in &self.0 {
            // print name
            let name = &x.name;
            write!(f, "{}", name)?;
            write!(f, "\t",)?;

            // print description
            let desc = x.description.as_ref().map(String::as_str).unwrap_or("-");
            write!(f, "{}", desc)?;
            write!(f, "\t",)?;

            // print language
            let lang = x.language.as_ref().map(String::as_str).unwrap_or("-");
            write!(f, "{}", lang)?;
            write!(f, "\t",)?;

            // print url
            let url = &x.html_url;
            write!(f, "{}", url)?;
            write!(f, "\n")?;
        }
        Ok(())
    }
}
