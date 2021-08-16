use anyhow::Result;
use futures::{future, stream::TryStreamExt};
use serde::{Deserialize, Serialize};
use shub::{
    client::GhClient,
    requests::UpdateRepository,
    responses::{Repository, WorkflowRun},
};
use std::path::Path;
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
                self.client
                    .actions()
                    .delete_workflow_run(owner, repo, id)
                    .await?;
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
        println!(
            "Downloading GitHub repository settings for {}/{} to {:?}.",
            owner, repo, path
        );
        let settings: RepositorySettings = self
            .client
            .repos()
            .get_repository(owner, repo)
            .await?
            .into();
        let buf = toml::to_vec(&settings)?;
        debug!(?settings, ?path, "writing settings");
        fs::write(path, &buf).await?;
        Ok(())
    }

    pub async fn apply_settings(&self, owner: Option<&str>, repo: &str, file: &Path) -> Result<()> {
        let owner = owner.unwrap_or(self.username);
        let path = file;
        println!(
            "Applying GitHub repository settings from {:?} for {}/{}.",
            path, owner, repo,
        );
        let settings = fs::read(path).await?;
        let settings: RepositorySettings = toml::from_slice(&settings)?;
        debug!(?settings, "applying settings");
        let settings = settings.into();
        self.client
            .repos()
            .update_repository(owner, repo, &settings)
            .await?;
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
