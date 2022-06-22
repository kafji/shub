mod app;
mod app2;
mod app_env;
mod cli;
mod commands;
mod database;
mod display;
mod github_client;
mod github_client2;
mod github_models;
mod repository_id;
mod types;

/// Run application;
pub use crate::app2::start as start_app;

use crate::github_models::{GhCommit, GhRepository};
use repository_id::FullRepoId;
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

#[derive(PartialEq, Clone, Debug)]
struct StarredRepository(GhRepository);

#[derive(PartialEq, Clone, Debug)]
struct OwnedRepository(GhRepository, Option<GhCommit>);
