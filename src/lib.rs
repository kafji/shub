mod app;
mod app2;
mod app_env;
mod cli;
mod commands;
mod display;
mod github_client;
mod github_client2;
mod github_models;
mod repo_id;

/// Run application;
pub use crate::app2::start as start_app;

use crate::github_models::{GhCommit, GhRepository};
use repo_id::FullRepoId;
use std::path::{Path, PathBuf};

fn create_local_repository_path(
    workspace_root_dir: impl AsRef<Path>,
    repo_id: &FullRepoId,
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
    let path = create_local_repository_path(workspace, &FullRepoId::new("kafji", "shub"));
    assert_eq!(path.display().to_string(), "./workspace/kafji/shub");
}

#[derive(PartialEq, Clone, Debug)]
struct StarredRepository(GhRepository);

#[derive(PartialEq, Clone, Debug)]
struct OwnedRepository(GhRepository, Option<GhCommit>);
