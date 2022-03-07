mod display;
mod github_client;
mod github_models;

pub mod app;
pub mod repository_id;

use crate::github_models::{GhCommit, GhRepository};
use repository_id::FullRepositoryId;
use std::path::{Path, PathBuf};

fn create_local_repository_path(
    workspace_root_dir: impl AsRef<Path>,
    repo_id: &FullRepositoryId,
) -> PathBuf {
    workspace_root_dir
        .as_ref()
        .to_path_buf()
        .join(&repo_id.owner)
        .join(&repo_id.name)
}

#[cfg(test)]
#[test]
fn test_local_repository_path() {
    let workspace = "./workspace";
    let path = create_local_repository_path(workspace, &FullRepositoryId::new("kafji", "shub"));
    assert_eq!(path.display().to_string(), "./workspace/kafji/shub");
}

#[derive(PartialEq, Clone, Debug)]
struct StarredRepository(GhRepository);

#[derive(PartialEq, Clone, Debug)]
struct OwnedRepository(GhRepository, Option<GhCommit>);
