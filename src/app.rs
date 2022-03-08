use crate::{
    create_local_repository_path, display::*, github_client::GitHubClientImpl, github_models::*,
    repository_id::PartialRepositoryId, FullRepositoryId, StarredRepository,
};
use anyhow::{bail, Context, Error};
use async_trait::async_trait;
use console::Term;
use dialoguer::Confirm;
use futures::{
    future,
    stream::{LocalBoxStream, TryStreamExt},
    FutureExt, Stream,
};
use git2::{build::RepoBuilder, Cred, FetchOptions, RemoteCallbacks};
use http::header::HeaderName;
use octocrab::Octocrab;
use sekret::Secret;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    env, fmt,
    io::Write,
    os::unix::prelude::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use tokio::{fs, task};
use tokio_stream::wrappers::ReadDirStream;

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct AppConfig<'a> {
    pub github_username: &'a str,
    pub github_token: Secret<&'a str>,
    pub workspace_root_dir: &'a Path,
}

#[derive(Debug)]
pub struct App<'a, GitHubClient> {
    github_username: &'a str,
    workspace_root_dir_path: &'a Path,
    github_client: GitHubClient,
    my_workspace_dir_path: PathBuf,
}

impl<'a> App<'a, GitHubClientImpl> {
    pub fn new(
        AppConfig {
            github_username,
            github_token,
            workspace_root_dir,
        }: AppConfig<'a>,
    ) -> Result<Self, Error> {
        let github_client =
            crate::github_client::GitHubClientImpl::new(github_token.map(ToOwned::to_owned))?;
        let my_workspace_dir_path = workspace_root_dir.join(github_username);
        let s = Self {
            github_username,
            workspace_root_dir_path: workspace_root_dir,
            github_client,
            my_workspace_dir_path,
        };
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

    pub async fn copy_repository_settings(
        &self,
        from: PartialRepositoryId,
        to: PartialRepositoryId,
    ) -> Result<(), Error> {
        let from = from.complete(self.github_username);
        let to = to.complete(self.github_username);

        let client = create_client()?;

        let get_settings = |repo_id: FullRepositoryId| {
            let client = client.clone();
            let FullRepositoryId { owner, name } = repo_id;
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

        let _: HashMap<String, Value> = {
            let FullRepositoryId { owner, name } = to;
            client
                .patch(format!("repos/{owner}/{name}"), Some(&new_settings))
                .await?
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

    pub async fn browse_upstream_repository(
        &'a self,
        repo_id: Option<PartialRepositoryId>,
    ) -> Result<(), Error> {
        let repo_id = match repo_id {
            Some(repo_id) => repo_id.complete(self.github_username),
            None => get_repo_id_for_cwd().await?,
        };

        let repo = self.github_client.get_repository(repo_id.clone()).await?;

        let url = {
            if !repo.fork.unwrap_or_default() {
                bail!("Repository {repo_id} is not a fork.")
            }
            repo.parent
                .and_then(|x| x.html_url)
                .expect("Forked repository should have the HTML URL to its parent repository.")
        };

        Command::new("xdg-open").arg(url.as_str()).status()?;

        Ok(())
    }

    pub async fn clone_repository(&'a self, repo_id: PartialRepositoryId) -> Result<(), Error> {
        let repo_id = repo_id.complete(self.github_username);

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

        let workspace_home = self.workspace_root_dir_path;
        let path = create_local_repository_path(workspace_home, &repo_id);
        println!(
            "Cloning {repo_id} repository to {path}.",
            path = path.display()
        );
        let repo = RepoBuilder::new()
            .fetch_options(create_fetch_options())
            .clone(&ssh_url, &path)
            .context("Failed to clone repository.")?;

        if let Some(upstream_url) = upstream_url {
            let mut remote = repo
                .remote("upstream", &upstream_url)
                .context("Failed to add upstream remote.")?;
            let mut options = {
                let mut opts = create_fetch_options();
                opts.prune(git2::FetchPrune::On);
                opts
            };
            remote
                .fetch(
                    &["+refs/heads/*:refs/remotes/origin/*"],
                    Some(&mut options),
                    None,
                )
                .context("Failed to fetch upstream.")?;
        }

        Ok(())
    }

    pub async fn poll_repository_build_status(
        &'a self,
        repo_id: Option<PartialRepositoryId>,
    ) -> Result<(), Error> {
        let mut out = Term::buffered_stdout();

        let repo_id = repo_id
            .map(|x| x.complete(self.github_username))
            .map(future::ok)
            .map(FutureExt::boxed)
            .unwrap_or_else(|| get_repo_id_for_cwd().boxed())
            .await?;

        writeln!(out, "{repo_id}\n")?;
        out.flush()?;

        let commit = self
            .github_client
            .list_repository_commits(&repo_id)
            .try_next()
            .await?
            .ok_or_else(|| {
                Error::msg(format!("Repository {repo_id} doesn't have a commit yet."))
            })?;

        writeln!(out, "{}", CommitInfo::from_github_commit(&commit))?;
        out.flush()?;

        loop {
            let runs = self
                .github_client
                .get_check_runs_for_gitref(&repo_id, &commit.sha)
                .await?;

            write!(out, "{}", BuildsInfo::from_github_check_runs(&runs))?;
            out.flush()?;

            let completed = runs.iter().map(|x| &x.completed_at).all(Option::is_some);
            if completed {
                break;
            }

            tokio::time::sleep(Duration::from_secs(10)).await;
            out.clear_last_lines(runs.len())?;
        }

        out.flush()?;
        Ok(())
    }

    pub async fn list_projects(&self) -> Result<(), Error> {
        let mut out = Term::buffered_stdout();

        let projects: Vec<_> = self.get_projects().await?.try_collect().await?;

        for project in projects {
            if let Some(name) = project.file_name() {
                writeln!(&mut out, "{}", name.to_string_lossy())?;
            }
        }
        out.flush()?;

        Ok(())
    }

    pub async fn edit_project(&self, project_name: &str) -> Result<(), Error> {
        let editor = env::var("SHUB_EDITOR")?;
        let path = self.get_project_path(project_name).await?;
        let error = Command::new(editor).arg(path).exec();
        Err(error.into())
    }

    pub async fn print_project_path(&self, project_name: &str) -> Result<(), Error> {
        let path = self.get_project_path(project_name).await?;
        println!("{}", path.display());
        Ok(())
    }

    async fn get_projects(
        &self,
    ) -> Result<impl Stream<Item = Result<PathBuf, std::io::Error>>, Error> {
        Ok(
            ReadDirStream::new(fs::read_dir(&self.my_workspace_dir_path).await?).try_filter_map(
                |entry| {
                    future::ok(
                        Some(entry.path()).and_then(|x| if x.is_dir() { Some(x) } else { None }),
                    )
                },
            ),
        )
    }

    async fn get_project_path(&self, project_name: &str) -> Result<PathBuf, Error> {
        self.get_projects()
            .await?
            .try_filter(|path| {
                future::ready(
                    path.file_name()
                        .and_then(|x| x.to_str())
                        .map(|x| x == project_name)
                        .unwrap_or_default(),
                )
            })
            .try_next()
            .await?
            .ok_or_else(|| Error::msg(format!("project `{project_name}` does not exists")))
    }

    pub async fn list_my_tasks(&'a self) -> Result<(), Error> {
        let mut out = Term::buffered_stdout();

        let issues: Vec<_> = self.github_client.list_user_issues().try_collect().await?;

        write!(out, "{}", TaskInfos::from_github_issues(&issues))?;
        out.flush()?;

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
        $repo
            .$key
            .ok_or_else(|| Error::msg(format!("Missing value for key `{}`.", stringify!($key))))
    };
}

impl ExtractRepositorySettings for GhRepository {
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
        write!(
            $w,
            "{key:>25} = {old:>5} -> {new:<5}\n",
            key = stringify!($key)
        )
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

async fn get_repo_id_for_cwd() -> Result<FullRepositoryId, Error> {
    task::block_in_place(|| {
        let repo = git2::Repository::discover(".")?;
        let origin = repo.find_remote("origin")?;
        let url = origin.url().unwrap();
        let start = url.find(':').unwrap();
        let end = url.find(".git").unwrap();
        let repo_id = &url[start + 1..end];
        let repo_id = repo_id.parse()?;
        Ok(repo_id)
    })
}

#[deprecated]
fn create_client() -> Result<Octocrab, Error> {
    let user_agent = concat!(
        env!("CARGO_PKG_NAME"),
        concat!("/", env!("CARGO_PKG_VERSION"))
    )
    .to_owned();
    let token = env::var("SHUB_TOKEN")?;
    let client = Octocrab::builder()
        .add_header(HeaderName::from_static("user-agent"), user_agent)
        .personal_token(token)
        .build()?;
    Ok(client)
}

#[async_trait]
pub trait GitHubClient<'a> {
    fn list_stared_repositories(&'a self) -> LocalBoxStream<'a, Result<GhRepository, Error>>;

    /// https://docs.github.com/en/rest/reference/commits#list-commits
    fn list_repository_commits<'b>(
        &'a self,
        repo_id: &'b FullRepositoryId,
    ) -> LocalBoxStream<'b, Result<GhCommit, Error>>
    where
        'a: 'b;

    /// https://docs.github.com/en/rest/reference/checks#list-check-runs-for-a-git-reference
    async fn get_check_runs_for_gitref<'b>(
        &'a self,
        repo_id: &'b FullRepositoryId,
        gitref: &'b str,
    ) -> Result<Vec<GhCheckRun>, Error>
    where
        'a: 'b;

    async fn get_repository(&'a self, repo_id: FullRepositoryId) -> Result<GhRepository, Error>;

    /// https://docs.github.com/en/rest/reference/issues#list-user-account-issues-assigned-to-the-authenticated-user
    fn list_user_issues(&'a self) -> LocalBoxStream<'a, Result<GhIssue, Error>>;
}
