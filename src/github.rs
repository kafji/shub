use crate::{
    app::{CheckRun, GitHubClient, GitHubCommit},
    RepositoryId, Secret,
};
use anyhow::{bail, Error};
use async_stream::try_stream;
use async_trait::async_trait;
use futures::{stream::LocalBoxStream, Future, Stream, StreamExt};
use http::header::HeaderName;
use octocrab::{models::Repository, Octocrab, Page};
use serde::Deserialize;
use std::{borrow::Cow, env, sync::Arc};

#[derive(Clone, Debug)]
pub struct GitHubClientImpl {
    client: Octocrab,
}

impl GitHubClientImpl {
    pub fn new(token: impl Into<Secret<String>>) -> Result<Self, Error> {
        let user_agent =
            concat!(env!("CARGO_PKG_NAME"), concat!("/", env!("CARGO_PKG_VERSION"))).to_owned();
        let token: Secret<_> = token.into();
        let client = Octocrab::builder()
            .add_header(HeaderName::from_static("user-agent"), user_agent)
            .personal_token(token.0)
            .build()?;
        let s = Self { client };
        Ok(s)
    }
}

#[async_trait]
impl<'a> GitHubClient<'a> for GitHubClientImpl {
    fn list_owned_repositories(&'a self) -> LocalBoxStream<'a, Result<Repository, Error>> {
        let this = self.clone();
        let items = unpage(move |page_num| {
            let client = this.client.clone();
            async move {
                let path: Cow<_> = if let Some(page_num) = page_num {
                    format!("user/repos?type=owner&sort=pushed&per_page=100&page={page_num}").into()
                } else {
                    "user/repos?type=owner&sort=pushed&per_page=100".into()
                };
                let items: Page<_> = client.get::<_, _, ()>(path, None).await?;
                Ok(items)
            }
        });
        items.boxed_local()
    }

    fn list_stared_repositories(&'a self) -> LocalBoxStream<'a, Result<Repository, Error>> {
        let this = self.clone();
        let items = unpage(move |page_num| {
            let client = this.client.clone();
            async move {
                let path: Cow<_> = if let Some(page_num) = page_num {
                    format!("user/starred?sort=updated&per_page=100&page={page_num}").into()
                } else {
                    "user/starred?sort=updated&per_page=100".into()
                };
                let items: Page<_> = client.get::<_, _, ()>(path, None).await?;
                Ok(items)
            }
        });
        items.boxed_local()
    }

    fn list_repository_commits<'b>(
        &'a self,
        repo_id: &'b RepositoryId,
    ) -> LocalBoxStream<'b, Result<GitHubCommit, Error>>
    where
        'a: 'b,
    {
        let items = unpage(move |page_num| async move {
            let RepositoryId { owner, name } = repo_id;
            let path = if let Some(page_num) = page_num {
                format!("repos/{owner}/{name}/commits?per_page=100&page={page_num}")
            } else {
                format!("repos/{owner}/{name}/commits?per_page=100")
            };
            let items: Page<_> = self.client.get::<_, _, ()>(path, None).await?;
            Ok(items)
        });
        items.boxed_local()
    }

    async fn get_check_runs_for_gitref<'b>(
        &'a self,
        repo_id: &'b RepositoryId,
        gitref: &'b str,
    ) -> Result<Vec<CheckRun>, Error>
    where
        'a: 'b,
    {
        let RepositoryId { owner, name } = repo_id;
        let path = format!("repos/{owner}/{name}/commits/{gitref}/check-runs?per_page=100");

        #[derive(Deserialize)]
        struct Envelope {
            check_runs: Vec<CheckRun>,
        }
        let res: Envelope = self.client.get::<_, _, ()>(path, None).await?;
        Ok(res.check_runs)
    }

    async fn get_repository(&'a self, repo_id: RepositoryId) -> Result<Repository, Error> {
        let client = &self.client;
        let repo = client.repos(&repo_id.owner, &repo_id.name).get().await;
        let repo = match repo {
            Ok(x) => x,
            Err(err) => {
                if matches!(&err, octocrab::Error::GitHub { source, .. } if source.message == "Not Found")
                {
                    bail!("Repository {repo_id} does not exist.")
                } else {
                    return Err(err.into());
                }
            }
        };
        Ok(repo)
    }

    async fn delete_repository(&'a self, repo_id: RepositoryId) -> Result<(), Error> {
        let client = &self.client;
        client.repos(repo_id.owner, repo_id.name).delete().await?;
        Ok(())
    }

    async fn fork_repository(&'a self, repo_id: RepositoryId) -> Result<(), Error> {
        let client = &self.client;
        client.repos(repo_id.owner, repo_id.name).create_fork().send().await?;
        Ok(())
    }
}

fn unpage<'a, T, F, Fut>(factory: F) -> impl Stream<Item = Result<T, Error>> + 'a
where
    T: Send + 'static,
    F: Fn(Option<u8>) -> Fut + 'a,
    Fut: Future<Output = Result<Page<T>, Error>> + 'a,
{
    try_stream! {
        let mut page_num = None;
        loop {
            let req = factory(page_num);
            let page = req.await?;
            let has_next = page.next.is_some();
            for repo in page {
                yield repo;
            }
            if !has_next {
                break;
            }
            page_num = (page_num.unwrap_or(1) + 1).into();
        }
    }
}
