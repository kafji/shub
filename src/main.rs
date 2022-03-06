mod cli;

use crate::cli::*;
use anyhow::{Error, Result};
use sekret::Secret;
use shub::app::{App, AppConfig};
use std::{env, path::PathBuf};
use tracing::debug;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_thread_ids(true)
        .init();

    let cmd = cli();

    let username = env::var("SHUB_USERNAME")?;
    let github_token = Secret(env::var("SHUB_TOKEN")?);
    let workspace_root_dir: PathBuf = env::var("WORKSPACE_HOME")?.into();

    let cfg = AppConfig {
        github_username: &username,
        github_token: github_token.as_ref().map(|x| x.as_str()),
        workspace_root_dir: &workspace_root_dir,
    };

    debug!(?cfg, ?cmd, "Starting.");

    let app = App::new(cfg)?;

    match cmd.cmd {
        Command::R { cmd } => match cmd {
            repos::Command::Clone { repo } => app.clone_repository(repo).await?,
            repos::Command::BrowseUpstream { repo } => app.browse_upstream_repository(repo).await?,
            repos::Command::BuildStatus { repo } => app.check_repository(repo).await?,
            repos::Command::ViewSettings { repo } => app.view_repository_settings(repo).await?,
            repos::Command::CopySettings { from, to } => {
                app.copy_repository_settings(from, to).await?
            }
        },
        Command::S { cmd } => match cmd {
            stars::Command::Ls => app.list_starred_repositories().await?,
        },
        Command::T { cmd } => match cmd {
            tasks::Command::Ls => todo!(),
        },
        Command::W { cmd } => match cmd {
            workspace::Command::Ls => app.list_projects().await?,
            workspace::Command::Edit { name } => app.edit_project(&name).await?,
            workspace::Command::Locate { name } => app.print_project_path(&name).await?,
        },
    };

    debug!("Exit.");
    Ok(())
}
