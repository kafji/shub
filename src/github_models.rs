use chrono::{DateTime, Utc};
use serde::Deserialize;

pub use octocrab::models::Repository as GhRepository;

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
