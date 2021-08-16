use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Default, Debug)]
pub struct UpdateRepository {
    pub allow_squash_merge: Option<bool>,
    pub allow_merge_commit: Option<bool>,
    pub allow_rebase_merge: Option<bool>,
    pub allow_auto_merge: Option<bool>,
    pub delete_branch_on_merge: Option<bool>,
}
