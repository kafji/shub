mod cli;

use crate::cli::*;
use anyhow::{Error, Result};
use shub::{
    app::{App, AppConfig},
    PartialRepositoryId,
};
use std::{env, path::PathBuf};
use tracing::debug;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_thread_ids(true)
        .init();

    let cmd = cmd();

    let username = env::var("SHUB_USERNAME")?;
    let github_token = env::var("SHUB_TOKEN")?;
    let workspace_root_dir: PathBuf = env::var("WORKSPACE_HOME")?.into();

    let cfg = AppConfig {
        username: &username,
        github_token: &github_token,
        workspace_root_dir: &workspace_root_dir,
    };

    debug!(?cfg, ?cmd, "Starting.");

    let app = App::new(cfg)?;

    match cmd.cmd {
        Commands::Repo { cmd } => match cmd {
            repo::Commands::Ls {} => app.list_owned_repositories().await?,
            repo::Commands::Open { repo, upstream } => app.open_repository(repo, upstream).await?,
            repo::Commands::Settings { cmd } => match cmd {
                repo::settings::Commands::View { repo } => {
                    app.view_repository_settings(repo).await?
                }
                repo::settings::Commands::Apply { from, to } => {
                    app.apply_repository_settings(from, to).await?
                }
            },
            repo::Commands::Fork { repo } => app.fork_repository(repo).await?,
            repo::Commands::Clone { repo } => app.clone_repository(repo).await?,
        },
        Commands::Star { cmd } => match cmd {
            star::Commands::Ls {} => app.list_starred_repositories().await?,
            star::Commands::Star { repo } => todo!(),
            star::Commands::Unstar { repo } => todo!(),
        },
        Commands::Git { cmd } => match cmd {
            zxc::Commands::Dump {} => app.git_dump().await?,
        },
    };

    debug!("Exit.");
    Ok(())
}
