use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Default, Clone, Debug)]
pub struct UpdateRepository {
    pub allow_squash_merge: Option<bool>,
    pub allow_merge_commit: Option<bool>,
    pub allow_rebase_merge: Option<bool>,
    pub allow_auto_merge: Option<bool>,
    pub delete_branch_on_merge: Option<bool>,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum RepositoryType {
    All,
    Owner,
    Public,
    Private,
    Member,
}

impl RepositoryType {
    pub const fn to_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Owner => "owner",
            Self::Public => "public",
            Self::Private => "private",
            Self::Member => "member",
        }
    }
}
