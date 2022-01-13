use serde::Deserialize;

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct ActionsRuns {
    pub total_count: i32,
    pub workflow_runs: Vec<WorkflowRun>,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct WorkflowRun {
    pub id: i32,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct Repository {
    pub id: i32,
    pub name: String,
    pub allow_rebase_merge: bool,
    pub allow_squash_merge: bool,
    pub allow_auto_merge: bool,
    pub delete_branch_on_merge: bool,
    pub allow_merge_commit: bool,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct MyRepository {
    pub id: i32,
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub archived: bool,
    pub visibility: RepositoryVisibility,
    pub fork: bool,
}

#[derive(Deserialize, PartialEq, Copy, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryVisibility {
    Public,
    Private,
}
