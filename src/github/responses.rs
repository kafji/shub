use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug)]
pub struct ActionsRuns {
    pub total_count: i32,
    pub workflow_runs: Vec<WorkflowRun>,
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct WorkflowRun {
    pub id: i32,
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct Repository {
    pub id: i32,
    pub name: String,
    pub allow_rebase_merge: bool,
    pub allow_squash_merge: bool,
    pub allow_auto_merge: bool,
    pub delete_branch_on_merge: bool,
    pub allow_merge_commit: bool,
}
