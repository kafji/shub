use crate::{
    app_env::AppEnv,
    database::Database,
    github_client2::GithubClient2,
    repository_id::{IsPartialRepositoryId, IsRepositoryId},
    types::{BuildStatus, Repository},
};
use anyhow::Error;
use futures::{future, StreamExt, TryStreamExt};
use octocrab::models::Repository as GhRepository;
use std::{
    cmp::{self, max},
    fmt,
};
use tracing::info;
use unicode_segmentation::UnicodeSegmentation;

/// Prints dashboard, repositories and their build statuses.
pub async fn print_dashboard<'app>(app_env: AppEnv<'app>) -> Result<(), Error> {
    let gh_username = app_env.github_username;

    let repos = app_env.database.get_dashboard_repositories(gh_username)?;
    let repos = repos
        .into_iter()
        .map(|r| {
            let bs = r.build_status.map(|x| x.to_string()).unwrap_or_default();
            (r.name, bs)
        })
        .collect::<Vec<_>>();
    let repos: Vec<_> = repos
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    do_print_dashboard(&repos[..]);

    Ok(())
}

pub async fn update_dashboard<'app>(mut env: AppEnv<'app>) -> Result<(), anyhow::Error> {
    let db = &mut env.database;
    let username = &env.github_username;
    let gh_client = env.github_client.clone();
    update_repositories(&gh_client, db).await?;
    update_build_statuses(db, username, gh_client).await?;

    print_dashboard(env).await?;

    Ok(())
}

/// Fetches owned repositories.
#[tracing::instrument(skip_all)]
async fn get_repositories<'a>(
    gh_client: &'a GithubClient2,
    gh_username: &'a str,
    db: &mut Database,
) -> Result<Vec<Repository>, anyhow::Error> {
    let repos = db.get_dashboard_repositories(gh_username)?;
    if !repos.is_empty() {
        info!("loaded repositories from database");
        return Ok(repos);
    }

    let predicate = move |r: GhRepository| {
        let owned = r.owner().map(|x| x == gh_username).unwrap_or_default();
        let a_fork = r.fork.unwrap_or_default();
        let archived = r.archived.unwrap_or_default();
        future::ok(if owned && !a_fork && !archived {
            Some(r)
        } else {
            None
        })
    };
    let gh_repos = gh_client
        .list_owned_repositories()
        .try_filter_map(predicate)
        .try_collect::<Vec<_>>()
        .await?;
    let repos = gh_repos
        .into_iter()
        .map(|x| Repository {
            name: x.name,
            owner: x
                .owner
                .map(|x| x.login)
                .unwrap_or_else(|| gh_username.to_owned()),
            a_fork: x.fork.unwrap_or_default(),
            archived: x.archived.unwrap_or_default(),
            build_status: None,
        })
        .collect::<Vec<_>>();
    db.put_repositories(&repos)?;
    Ok(repos)
}

/// Fetches build status.
async fn get_build_status(
    gh_client: &GithubClient2,
    repo_id: &(impl IsRepositoryId + fmt::Debug),
) -> Result<Option<BuildStatus>, Error> {
    let commit = gh_client.get_latest_commit(repo_id).await?;
    let runs = match commit {
        Some(commit) => {
            let gitref = &commit.sha;
            let runs = gh_client.get_check_runs_for_gitref(repo_id, gitref).await?;
            Some(runs)
        }
        None => None,
    };
    let status = if let Some(runs) = runs {
        runs.iter()
            .map(|x| match x.status.as_str() {
                "queued" => None,
                "in_progress" => Some(BuildStatus::InProgress),
                "completed" => match x.conclusion.as_deref() {
                    Some("success") => Some(BuildStatus::Success),
                    _ => Some(BuildStatus::Failure),
                },
                _ => Some(BuildStatus::Failure),
            })
            .reduce(|acc, x| max(acc, x))
    } else {
        None
    };
    Ok(status.flatten())
}

fn do_print_dashboard<'a>(xs: &[(&'a str /* name */, &'a str /* build status */)]) {
    // cache name lengths
    let mut name_lengths = Vec::with_capacity(xs.len());

    // find max length for name
    let mut name_max_length = 0;

    for (name, _) in xs {
        let length = name.graphemes(true).count();
        name_lengths.push(length);
        name_max_length = cmp::max(name_max_length, length);
    }

    let default_col_margin = 2;

    // print dashboard
    for (idx, (name, build_status)) in xs.iter().enumerate() {
        // calc. how many spaces required to align the next column
        let name_col_right_margin = {
            let pad_deficit = name_max_length - name_lengths[idx];
            (0..pad_deficit + default_col_margin)
                .map(|_| ' ')
                .collect::<String>()
        };

        println!("{}{}{}", name, name_col_right_margin, build_status);
    }
}

async fn update_repositories(
    gh_client: &GithubClient2,
    db: &mut Database,
) -> Result<(), anyhow::Error> {
    info!("updating repositories");

    // fetch owned repositories
    let gh_repos = gh_client
        .list_owned_repositories()
        .try_collect::<Vec<_>>()
        .await?;

    // update stored repositories
    let repos = gh_repos
        .into_iter()
        .map(Repository::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    db.put_repositories(&repos[..])?;

    Ok(())
}

async fn update_build_statuses(
    db: &mut Database,
    owner: &str,
    gh_client: GithubClient2,
) -> Result<(), anyhow::Error> {
    info!("updating build statuses");

    // get stored repositories
    let repos = db.get_dashboard_repositories(owner)?;

    // fetch build statuses
    let bss = {
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        tokio::spawn(async move {
            futures::stream::iter(repos)
                .then(|x| futures::future::ok::<_, anyhow::Error>(x))
                .and_then(move |x| {
                    let gh_client = gh_client.clone();
                    async move {
                        let build_status = get_build_status(&gh_client, &x).await?;
                        info!("build status: {:?}", build_status);
                        Ok((x, build_status))
                    }
                })
                .try_for_each_concurrent(2, move |(r, s)| {
                    let tx = tx.clone();
                    async move {
                        if let Some(s) = s {
                            tx.send((r, s)).await?;
                        }
                        Ok(())
                    }
                })
                .await
        });
        let collector = tokio::spawn(async move {
            let mut vs = Vec::new();
            loop {
                let v = rx.recv().await;
                match v {
                    Some(x) => vs.push(x),
                    None => break,
                }
            }
            Result::<_, anyhow::Error>::Ok(vs)
        });
        collector
    }
    .await??;

    // update stored values
    db.set_build_statuses(&bss[..])?;

    Ok(())
}
