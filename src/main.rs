mod cli;

use crate::cli::*;
use anyhow::{Error, Result};
use shub::{
    app::{App, AppConfig},
    Secret,
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
        Commands::Repos { cmd } => match cmd {
            repos::Commands::Ls {} => app.list_owned_repositories().await?,
            repos::Commands::Open { repo, upstream } => app.open_repository(repo, upstream).await?,
            repos::Commands::Settings { cmd } => match cmd {
                repos::settings::Commands::View { repo } => {
                    app.view_repository_settings(repo).await?
                }
                repos::settings::Commands::Apply { from, to } => {
                    app.apply_repository_settings(from, to).await?
                }
            },
            repos::Commands::Fork { repo } => app.fork_repository(repo).await?,
            repos::Commands::Clone { repo } => app.clone_repository(repo).await?,
            repos::Commands::Create { repo } => todo!(),
            repos::Commands::Delete { repo } => app.delete_repository(repo).await?,
            repos::Commands::Status { repo } => app.check_repository(repo).await?,
        },
        Commands::Stars { cmd } => match cmd {
            stars::Commands::Ls {} => app.list_starred_repositories().await?,
            stars::Commands::Star { repo } => todo!(),
            stars::Commands::Unstar { repo } => todo!(),
        },
        Commands::Workspace { cmd } => todo!(),
    };

    debug!("Exit.");
    Ok(())
}
