use crate::repo_id::PartialRepoId2;
use chrono::{DateTime, Utc};
use serde::Deserialize;

pub use octocrab::models::Repository as GhRepository;

impl PartialRepoId2 for GhRepository {
    fn owner(&self) -> Option<&str> {
        self.owner.as_ref().map(|x| x.login.as_str())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct GhCommit {
    pub sha: String,
    pub commit: GhCommitDetail,
    pub author: Option<GhUser>,
    pub committer: Option<GhUser>,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct GhCommitDetail {
    pub author: GhCommitActor,
    pub committer: GhCommitActor,
    pub message: String,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct GhCommitActor {
    pub name: Option<String>,
    pub email: Option<String>,
    pub date: DateTime<Utc>,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
#[non_exhaustive]
pub struct GhUser {
    pub login: String,
    pub id: u64,
    pub r#type: String,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct GhCheckRun {
    pub id: u64,
    pub head_sha: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub output: Option<GhCheckRunOutput>,
    pub name: String,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct GhCheckRunOutput {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub text: Option<String>,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct GhIssue {
    #[serde(flatten)]
    pub inner: octocrab::models::issues::Issue,

    pub repository: GhIssueRepository,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct GhIssueRepository {
    pub name: String,
    pub full_name: String,
}
