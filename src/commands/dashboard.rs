use crate::{
    app_env::AppEnv,
    github_client2::GithubClient2,
    repo_id::{PartialRepoId2, RepoId},
};
use anyhow::Error;
use futures::{future, Stream, TryStreamExt};
use octocrab::models::Repository as GhRepository;
use std::{borrow::Cow, cmp::max, fmt};

fn get_repositories<'a>(
    gh_client: &'a GithubClient2,
    gh_username: &'a str,
) -> impl Stream<Item = Result<GhRepository, Error>> + 'a {
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
    gh_client
        .list_owned_repositories()
        .try_filter_map(predicate)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum BuildStatus {
    Success,
    InProgress,
    Failure,
}

impl BuildStatus {
    fn from_str(str: &str) -> Self {
        match str {
            "completed" => Self::Success,
            "in_progress" => Self::InProgress,
            "failure" => Self::Failure,
            _ => panic!("unexpected str, was `{}`", str),
        }
    }
}

impl fmt::Display for BuildStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BuildStatus::Success => "Success",
            BuildStatus::InProgress => "In progress",
            BuildStatus::Failure => "Failure",
        })
    }
}

async fn get_build_status(
    gh_client: &GithubClient2,
    repo_id: &impl RepoId,
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
            .map(|x| BuildStatus::from_str(&x.status))
            .reduce(|acc, x| max(acc, x))
    } else {
        None
    };
    Ok(status)
}

struct RepositoryDisplay {
    name: String,
    build_status: Option<BuildStatus>,
}

impl fmt::Display for RepositoryDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let build_status = self
            .build_status
            .map(|x| x.to_string().into())
            .unwrap_or_else(|| Cow::Borrowed("None"));
        write!(f, "{}\t{}", self.name, build_status)
    }
}

impl From<(GhRepository, Option<BuildStatus>)> for RepositoryDisplay {
    fn from((repo, status): (GhRepository, Option<BuildStatus>)) -> Self {
        let name = repo.name;
        let build_status = status;
        Self { name, build_status }
    }
}

pub async fn print_dashboard<'app>(app_env: AppEnv<'app>) -> Result<(), Error> {
    let gh_client = app_env.github_client();
    let repos = get_repositories(gh_client, app_env.github_username())
        .and_then(|r| async {
            let repo_id = r.into_full(app_env.github_username());
            let status = get_build_status(gh_client, &repo_id).await?;
            Ok((r, status))
        })
        .map_ok(Into::<RepositoryDisplay>::into)
        .try_collect::<Vec<_>>()
        .await?;
    for repo in repos {
        println!("{}", repo);
    }
    Ok(())
}
