mod cli;

use crate::cli::*;
use anyhow::{Error, Result};
use futures::{future, stream, StreamExt, TryStreamExt};
use shub::{app::App, PartialRepositoryId};
use std::{env, sync::Arc};
use tracing::debug;
use tracing_subscriber::EnvFilter;

// async fn download_settings(
//     app: App<'_>,
//     DownloadSettings { repository, file }: DownloadSettings,
// ) -> Result<()> {
//     let Repository { owner, name } = repository;
//     app.download_settings(owner.as_ref().map(String::as_str), &name, file.as_path()).await?;
//     Ok(())
// }

// async fn apply_settings<'a>(
//     app: App<'a>,
//     ApplySettings { file, repository, repositories }: ApplySettings,
// ) -> Result<()> {
//     let app = Arc::new(app);
//     let file = file.as_path();
//     stream::once(future::ready(repository))
//         .chain(stream::iter(repositories))
//         .map(Result::<_, anyhow::Error>::Ok)
//         .try_for_each_concurrent(
//             None,
//             (|app: Arc<App<'a>>| {
//                 move |Repository { owner, name }| {
//                     let app = app.clone();
//                     async move {
//                         let owner = owner.as_ref().map(String::as_str);
//                         app.apply_settings(owner, &name, file).await?;
//                         Ok(())
//                     }
//                 }
//             })(app.clone()),
//         )
//         .await?;
//     Ok(())
// }

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_thread_ids(true)
        .init();

    let cmd = cmd();

    let username = env::var("SHUB_USERNAME")?;
    let token = env::var("SHUB_TOKEN")?;

    debug!(?username, ?cmd, "Starting.");

    let app = App::new(&username, &token)?;

    match cmd.cmd {
        Commands::Repo { cmd } => match cmd {
            repo::Commands::Ls {} => app.list_owned_repositories().await?,
            repo::Commands::Open { repo: PartialRepositoryId { owner, name }, upstream } => {
                app.open_repository(owner.as_ref().map(|x| x.as_str()), &name, upstream).await?
            }
            repo::Commands::Settings { cmd } => match cmd {
                repo::settings::Commands::Get { repo } => app.get_repository_settings(repo).await?,
                repo::settings::Commands::Apply { from, to } => {
                    app.apply_repository_settings(from, to).await?
                }
            },
            repo::Commands::Fork { repo } => app.fork_repository(repo).await?,
            repo::Commands::Clone { repo } => app.clone_repository(repo).await?,
        },
        Commands::Star { cmd } => match cmd {
            star::Commands::Ls {} => app.list_starred_repositories().await?,
        },
        Commands::Git { cmd } => match cmd {
            zxc::Commands::Dump {} => app.git_dump().await?,
        },
    };

    debug!("Exit.");
    Ok(())
}
