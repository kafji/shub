use crate::{
    github::GitHubClientImpl, local_repository_path, GetRepositoryId, OwnedRepository,
    PartialRepositoryId, RepositoryId, Secret, StarredRepository,
};
use anyhow::{bail, ensure, Context, Error, Result};
use async_trait::async_trait;
use dialoguer::Confirm;
use futures::{
    future,
    stream::{LocalBoxStream, TryStreamExt},
    StreamExt,
};
use git2::{build::RepoBuilder, Cred, FetchOptions, RemoteCallbacks};
use http::header::HeaderName;
use indoc::formatdoc;
use octocrab::{models::Repository as GitHubRepository, Octocrab};
use serde::{Deserialize, Serialize};
use std::{env, fmt, path::Path, process::Command};

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct AppConfig<'a> {
    pub github_username: &'a str,
    pub github_token: Secret<&'a str>,
    pub workspace_root_dir: &'a Path,
}

#[derive(Debug)]
pub struct App<'a, GitHubClient> {
    github_username: &'a str,
    workspace_root_dir: &'a Path,
    github_client: GitHubClient,
}

impl<'a> App<'a, GitHubClientImpl> {
    pub fn new(
        AppConfig { github_username, github_token, workspace_root_dir }: AppConfig<'a>,
    ) -> Result<Self, Error> {
        let github_client =
            crate::github::GitHubClientImpl::new(github_token.map(ToOwned::to_owned))?;
        let s = Self { github_username, workspace_root_dir, github_client };
        Ok(s)
    }
}

impl<'a, GitHubClient> App<'a, GitHubClient>
where
    GitHubClient: self::GitHubClient<'a>,
{
    pub async fn view_repository_settings(
        &'a self,
        repo_id: PartialRepositoryId,
    ) -> Result<(), Error> {
        let repo_id = repo_id.complete(self.github_username);
        let repo = self.github_client.get_repository(repo_id).await?;
        let settings = repo.extract_repository_settings()?;
        println!("{}", settings);
        Ok(())
    }

    pub async fn apply_repository_settings(
        &self,
        from: PartialRepositoryId,
        to: PartialRepositoryId,
    ) -> Result<(), Error> {
        let from = from.complete(self.github_username);
        let to = to.complete(self.github_username);

        let client = create_client()?;

        let get_settings = |repo_id: RepositoryId| {
            let client = client.clone();
            let RepositoryId { owner, name } = repo_id;
            let owner = owner.to_owned();
            let name = name.to_owned();
            async move {
                let repo = client.repos(owner, name).get().await?;
                let settings = repo.extract_repository_settings()?;
                Result::<_, Error>::Ok(settings)
            }
        };

        let old_settings = get_settings(to.clone()).await?;
        let new_settings = get_settings(from.clone()).await?;
        let diff = RepositorySettingsDiff::new(&old_settings, &new_settings);

        println!("{}", diff);

        if !Confirm::new()
            .with_prompt("Apply settings?")
            .default(false)
            .show_default(true)
            .wait_for_newline(true)
            .interact()?
        {
            return Ok(());
        }

        let _: GitHubRepository = {
            let RepositoryId { owner, name } = to;
            client.patch(format!("repos/{owner}/{name}"), Some(&new_settings)).await?
        };

        Ok(())
    }

    pub async fn list_starred_repositories(&'a self) -> Result<(), Error> {
        let repos = self.github_client.list_stared_repositories();
        repos
            .map_ok(StarredRepository)
            .try_for_each(|repo| {
                println!("{}", repo);
                future::ok(())
            })
            .await?;
        Ok(())
    }

    pub async fn open_repository(
        &'a self,
        repo_id: PartialRepositoryId,
        upstream: bool,
    ) -> Result<(), Error> {
        let repo_id = repo_id.complete(self.github_username);

        let repo = self.github_client.get_repository(repo_id.clone()).await?;

        let url = if upstream {
            if !repo.fork.unwrap_or_default() {
                bail!("Repository {repo_id} is not a fork.")
            }
            repo.parent
                .map(|x| x.html_url)
                .flatten()
                .expect("Forked repository should have the HTML URL to its parent repository.")
        } else {
            repo.html_url.expect("Repository should have the HTML URL to itself.")
        };

        Command::new("xdg-open").arg(url.as_str()).status()?;
        Ok(())
    }

    pub async fn list_owned_repositories(&'a self) -> Result<(), Error> {
        let repos = self.github_client.list_owned_repositories();
        repos
            .and_then(|repo| async {
                let repo_id = repo.get_repository_id()?;
                let commits: Vec<_> = self
                    .github_client
                    .list_repository_commits(repo_id)
                    .take(1)
                    .try_collect()
                    .await?;
                let commit = commits.first().map(ToOwned::to_owned);
                Ok(OwnedRepository(repo, commit))
            })
            .try_for_each(|repo| {
                println!("{}", repo);
                future::ok(())
            })
            .await?;
        Ok(())
    }

    pub async fn fork_repository(&self, repo_id: RepositoryId) -> Result<(), Error> {
        let client = create_client()?;
        client.repos(&repo_id.owner, &repo_id.name).create_fork().send().await?;
        Ok(())
    }

    pub async fn clone_repository(&'a self, repo_id: PartialRepositoryId) -> Result<(), Error> {
        let repo_id = repo_id.complete(self.github_username);

        ensure!(repo_id.owner != self.github_username);

        let repo_info = self.github_client.get_repository(repo_id.clone()).await?;

        let ssh_url = repo_info
            .ssh_url
            .ok_or_else(|| Error::msg("Expecting repository to have ssh url, but was not."))?;

        let upstream_url = match repo_info.parent {
            Some(upstream) => upstream
                .ssh_url
                .ok_or_else(|| {
                    Error::msg("Expecting upstream repository to have ssh url, but was not.")
                })?
                .into(),
            None => None,
        };

        let workspace_home = env::var("WORKSPACE_HOME")?;
        let path = local_repository_path(workspace_home, &repo_id);
        println!("Cloning {repo_id} repository to {path}.", path = path.display());
        let repo = RepoBuilder::new()
            .fetch_options(create_fetch_options())
            .clone(&ssh_url, &path)
            .context("Failed to clone repository.")?;

        if let Some(upstream_url) = upstream_url {
            let mut remote =
                repo.remote("upstream", &upstream_url).context("Failed to add upstream remote.")?;
            let mut options = {
                let mut opts = create_fetch_options();
                opts.prune(git2::FetchPrune::On);
                opts
            };
            remote
                .fetch(&["+refs/heads/*:refs/remotes/origin/*"], Some(&mut options), None)
                .context("Failed to fetch upstream.")?;
        }

        Ok(())
    }

    pub async fn delete_repository(&'a self, repo_id: PartialRepositoryId) -> Result<(), Error> {
        let repo_id = repo_id.complete(self.github_username);
        let repo = self.github_client.get_repository(repo_id.clone()).await?;
        ensure!(repo.fork.unwrap_or_default());
        self.github_client.delete_repository(repo_id).await?;
        Ok(())
    }
}

fn create_fetch_options<'a>() -> FetchOptions<'a> {
    let mut opts = FetchOptions::new();
    opts.remote_callbacks(create_remote_callbacks());
    opts
}

fn create_remote_callbacks<'a>() -> RemoteCallbacks<'a> {
    let mut cbs = RemoteCallbacks::new();
    cbs.credentials(|_url, username_from_url, _credential_type| {
        let username = username_from_url.unwrap_or("git");
        Cred::ssh_key_from_agent(username)
    });
    cbs
}

trait ExtractRepositorySettings {
    fn extract_repository_settings(&self) -> Result<RepositorySettings, Error>;
}

#[derive(Deserialize, Serialize, PartialEq, Copy, Clone, Debug)]
struct RepositorySettings {
    allow_rebase_merge: bool,
    allow_squash_merge: bool,
    allow_auto_merge: bool,
    delete_branch_on_merge: bool,
    allow_merge_commit: bool,
}

macro_rules! write_key {
    ($w:expr, $this:expr, $key:ident) => {{
        let val = $this.$key;
        write!($w, "{key:>25} = {val:5}\n", key = stringify!($key))
    }};
}

impl fmt::Display for RepositorySettings {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_key!(f, self, allow_rebase_merge)?;
        write_key!(f, self, allow_squash_merge)?;
        write_key!(f, self, allow_auto_merge)?;
        write_key!(f, self, delete_branch_on_merge)?;
        write_key!(f, self, allow_merge_commit)?;
        Ok(())
    }
}

macro_rules! extract_key {
    ($repo:expr, $key:ident) => {
        $repo.$key.ok_or_else(|| {
            Error::msg(formatdoc!("Missing value for key `{key}`.", key = stringify!($key)))
        })
    };
}

impl ExtractRepositorySettings for GitHubRepository {
    fn extract_repository_settings(&self) -> Result<RepositorySettings, Error> {
        let repo = self;
        let s = RepositorySettings {
            allow_rebase_merge: extract_key!(repo, allow_rebase_merge)?,
            allow_squash_merge: extract_key!(repo, allow_squash_merge)?,
            allow_auto_merge: extract_key!(repo, allow_auto_merge)?,
            delete_branch_on_merge: extract_key!(repo, delete_branch_on_merge)?,
            allow_merge_commit: extract_key!(repo, allow_merge_commit)?,
        };
        Ok(s)
    }
}

#[derive(PartialEq, Clone, Debug)]
struct RepositorySettingsDiff<'a> {
    old: &'a RepositorySettings,
    new: &'a RepositorySettings,
}

impl<'a> RepositorySettingsDiff<'a> {
    fn new(old: &'a RepositorySettings, new: &'a RepositorySettings) -> Self {
        Self { old, new }
    }
}

macro_rules! diff_key {
    ($w:expr, $this:expr, $key:ident) => {{
        let old = $this.old.$key;
        let new = $this.new.$key;
        write!($w, "{key:>25} = {old:>5} -> {new:<5}\n", key = stringify!($key))
    }};
}

impl fmt::Display for RepositorySettingsDiff<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        diff_key!(f, self, allow_rebase_merge)?;
        diff_key!(f, self, allow_squash_merge)?;
        diff_key!(f, self, allow_auto_merge)?;
        diff_key!(f, self, delete_branch_on_merge)?;
        diff_key!(f, self, allow_merge_commit)?;
        Ok(())
    }
}

#[deprecated]
fn create_client() -> Result<Octocrab, Error> {
    let user_agent =
        concat!(env!("CARGO_PKG_NAME"), concat!("/", env!("CARGO_PKG_VERSION"))).to_owned();
    let token = env::var("SHUB_TOKEN")?;
    let client = Octocrab::builder()
        .add_header(HeaderName::from_static("user-agent"), user_agent)
        .personal_token(token)
        .build()?;
    Ok(client)
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct GitHubCommit {
    pub commit: GitHubCommitDetail,
    pub author: Option<GitHubCommitActor>,
    pub committer: Option<GitHubCommitActor>,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct GitHubCommitDetail {
    pub message: String,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
#[non_exhaustive]
pub struct GitHubCommitActor {
    pub login: String,
    pub id: u32,
    pub r#type: String,
}

#[async_trait]
pub trait GitHubClient<'a> {
    fn list_owned_repositories(&'a self) -> LocalBoxStream<'a, Result<GitHubRepository, Error>>;

    fn list_stared_repositories(&'a self) -> LocalBoxStream<'a, Result<GitHubRepository, Error>>;

    fn list_repository_commits(
        &'a self,
        repo_id: RepositoryId,
    ) -> LocalBoxStream<'a, Result<GitHubCommit, Error>>;

    async fn get_repository(&'a self, repo_id: RepositoryId) -> Result<GitHubRepository, Error>;

    async fn delete_repository(&'a self, repo_id: RepositoryId) -> Result<(), Error>;
}
