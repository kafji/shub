#![deny(rust_2018_idioms)]

use crate::{app::App, cli::*};
use anyhow::Result;
use futures::{future, TryStreamExt};
use futures::{stream, StreamExt};
use shub::{GhClient, PersonalAccessToken};
use std::{env, sync::Arc};
use tracing::debug;
use tracing_subscriber::EnvFilter;

mod app;
mod cli;

async fn delete_all_workflow_runs(
    app: App<'_>,
    DeleteRuns { repository }: DeleteRuns,
) -> Result<()> {
    let Repository { owner, name } = repository;
    let owner = owner.as_ref().map(String::as_str);
    app.delete_all_workflow_runs(owner, &name).await?;
    Ok(())
}

async fn download_settings(
    app: App<'_>,
    DownloadSettings { repository, file }: DownloadSettings,
) -> Result<()> {
    let Repository { owner, name } = repository;
    app.download_settings(owner.as_ref().map(String::as_str), &name, file.as_path()).await?;
    Ok(())
}

async fn apply_settings<'a>(
    app: App<'a>,
    ApplySettings { file, repository, repositories }: ApplySettings,
) -> Result<()> {
    let app = Arc::new(app);
    let file = file.as_path();
    stream::once(future::ready(repository))
        .chain(stream::iter(repositories))
        .map(Result::<_, anyhow::Error>::Ok)
        .try_for_each_concurrent(
            None,
            (|app: Arc<App<'a>>| {
                move |Repository { owner, name }| {
                    let app = app.clone();
                    async move {
                        let owner = owner.as_ref().map(String::as_str);
                        app.apply_settings(owner, &name, file).await?;
                        Ok(())
                    }
                }
            })(app.clone()),
        )
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    let cmd = cli::cmd();
    debug!(?cmd, "launched");

    // create app
    let username = env::var("SHUB_USERNAME")?;
    let token = env::var("SHUB_TOKEN")?;
    let token = PersonalAccessToken::new(&username, &token);
    let client = GhClient::new(None, &token)?;
    let app = App { username: &username, client };

    // process command
    use Subcommand::*;
    match cmd.cmd {
        Actions(cmd) => {
            use ActionsSubcommand::*;
            match cmd.cmd {
                DeleteRuns(cmd) => delete_all_workflow_runs(app, cmd).await?,
            }
        }
        Repos(cmd) => {
            use ReposSubcommand::*;
            match cmd.cmd {
                DownloadSettings(cmd) => download_settings(app, cmd).await?,
                ApplySettings(cmd) => apply_settings(app, cmd).await?,
            }
        }
        Stars(cmd) => app.list_starred(cmd.lang.as_ref(), cmd.short).await?,
    };

    debug!("exiting");
    Ok(())
}
