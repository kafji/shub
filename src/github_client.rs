use crate::{app::GitHubClient, github_models::*, FullRepositoryId};
use anyhow::{bail, Error};
use async_stream::try_stream;
use async_trait::async_trait;
use futures::{
    stream::{self, LocalBoxStream},
    Future, Stream, StreamExt, TryStreamExt,
};
use http::header::HeaderName;
use octocrab::{Octocrab, Page};
use sekret::Secret;
use serde::Deserialize;
use std::{borrow::Cow, env};

#[derive(Clone, Debug)]
pub struct GitHubClientImpl {
    client: Octocrab,
}

impl GitHubClientImpl {
    pub fn new(token: impl Into<Secret<String>>) -> Result<Self, Error> {
        let user_agent = concat!(
            env!("CARGO_PKG_NAME"),
            concat!("/", env!("CARGO_PKG_VERSION"))
        )
        .to_owned();
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
    fn list_stared_repositories(&'a self) -> LocalBoxStream<'a, Result<GhRepository, Error>> {
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
        repo_id: &'b FullRepositoryId,
    ) -> LocalBoxStream<'b, Result<GhCommit, Error>>
    where
        'a: 'b,
    {
        let items = unpage(move |page_num| async move {
            let FullRepositoryId { owner, name } = repo_id;
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
        repo_id: &'b FullRepositoryId,
        gitref: &'b str,
    ) -> Result<Vec<GhCheckRun>, Error>
    where
        'a: 'b,
    {
        let FullRepositoryId { owner, name } = repo_id;
        let path = format!("repos/{owner}/{name}/commits/{gitref}/check-runs?per_page=100");

        #[derive(Deserialize)]
        struct Envelope {
            check_runs: Vec<GhCheckRun>,
        }
        let res: Envelope = self.client.get::<_, _, ()>(path, None).await?;
        Ok(res.check_runs)
    }

    async fn get_repository(&'a self, repo_id: FullRepositoryId) -> Result<GhRepository, Error> {
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

    fn list_user_issues(&'a self) -> LocalBoxStream<'a, Result<GhIssue, Error>> {
        stream::try_unfold::<PageNum, _, _, Page<GhIssue>>(
            PageNum::Init,
            move |page_num| async move {
                let path: Option<Cow<str>> = match page_num {
                    PageNum::Init => Some("issues".into()),
                    PageNum::Num(x) => Some(format!("issues?per_page=100&page={x}").into()),
                    PageNum::End => None,
                };
                match path {
                    Some(path) => {
                        let page: Page<GhIssue> = self.client.get::<_, _, ()>(path, None).await?;
                        let next_page_num = page
                            .next
                            .as_ref()
                            .map(|_| page_num.succ())
                            .unwrap_or(PageNum::End);
                        Ok(Some((page, next_page_num)))
                    }
                    None => Result::<_, Error>::Ok(None),
                }
            },
        )
        .map_ok(|x: Page<GhIssue>| {
            let x: Vec<_> = x.into_iter().collect();
            stream::iter(x).map(Ok)
        })
        .try_flatten()
        .boxed_local()
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
enum PageNum {
    Init,
    Num(u8),
    End,
}

impl PageNum {
    fn succ(self) -> PageNum {
        let num = match self {
            PageNum::Init => 1,
            PageNum::Num(x) => x + 1,
            PageNum::End => panic!(),
        };
        PageNum::Num(num)
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
