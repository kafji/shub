use crate::{
    app::{GitHubClient, GitHubCommit},
    RepositoryId,
};
use anyhow::Error;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::{stream::LocalBoxStream, Future, Stream, StreamExt};
use http::header::HeaderName;
use octocrab::models::Repository;
use octocrab::{Octocrab, Page};
use std::{borrow::Cow, env};

#[derive(Clone, Debug)]
pub struct GitHubClientImpl {
    client: Octocrab,
}

impl GitHubClientImpl {
    pub fn new() -> Result<Self, Error> {
        let user_agent =
            concat!(env!("CARGO_PKG_NAME"), concat!("/", env!("CARGO_PKG_VERSION"))).to_owned();
        let token = env::var("SHUB_TOKEN")?;
        let client = Octocrab::builder()
            .add_header(HeaderName::from_static("user-agent"), user_agent)
            .personal_token(token)
            .build()?;
        let s = Self { client };
        Ok(s)
    }
}

#[async_trait]
impl<'a> GitHubClient<'a> for GitHubClientImpl {
    fn list_owned_repositories(&'a self) -> LocalBoxStream<'a, Result<Repository, Error>> {
        let this = self.clone();
        let repos = unpage(Box::new(move |page_num| {
            let client = this.client.clone();
            async move {
                let path: Cow<_> = if let Some(page_num) = page_num {
                    format!("user/repos?type=owner&sort=pushed&per_page=100&page={page_num}").into()
                } else {
                    "user/repos?type=owner&sort=pushed&per_page=100".into()
                };
                let repos: Page<Repository> = client.get::<_, _, ()>(path, None).await?;
                Ok(repos)
            }
        }));
        repos.boxed_local()
    }

    fn list_stared_repositories(&'a self) -> LocalBoxStream<'a, Result<Repository, Error>> {
        let this = self.clone();
        let repos = unpage(Box::new(move |page_num| {
            let client = this.client.clone();
            async move {
                let path: Cow<_> = if let Some(page_num) = page_num {
                    format!("user/starred?sort=updated&per_page=100&page={page_num}").into()
                } else {
                    "user/starred?sort=updated&per_page=100".into()
                };
                let repos: Page<Repository> = client.get::<_, _, ()>(path, None).await?;
                Ok(repos)
            }
        }));
        repos.boxed_local()
    }

    fn list_repository_commits(
        &'a self,
        repo_id: RepositoryId,
    ) -> LocalBoxStream<'a, Result<GitHubCommit, Error>> {
        let this = self.clone();
        let repos = unpage(Box::new(move |page_num| {
            let client = this.client.clone();
            let repo_id = repo_id.clone();
            async move {
                let RepositoryId { owner, name } = repo_id;
                let path = if let Some(page_num) = page_num {
                    format!("repos/{owner}/{name}/commits?page={page_num}")
                } else {
                    format!("repos/{owner}/{name}/commits?sort=updated")
                };
                let repos: Page<GitHubCommit> = client.get::<_, _, ()>(path, None).await?;
                Ok(repos)
            }
        }));
        repos.boxed_local()
    }
}

fn unpage<'a, T, F>(
    factory: Box<dyn Fn(Option<u8>) -> F>,
) -> impl Stream<Item = Result<T, Error>> + 'a
where
    T: 'a + Send,
    F: 'a + Future<Output = Result<Page<T>, Error>>,
{
    try_stream! {
        let mut page_num = None;
        loop {
            let req = (factory)(page_num);
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
