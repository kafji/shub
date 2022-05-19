use crate::{
    github_models::{GhCheckRun, GhCommit, GhRepository},
    repo_id::RepoId,
};
use anyhow::Error;
use futures::{stream, Stream, StreamExt, TryStreamExt};
use http::header::HeaderName;
use octocrab::{Octocrab, Page};
use sekret::Secret;
use serde::Deserialize;

const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    concat!("/", env!("CARGO_PKG_VERSION"))
);

/// Defines a higher level queries to GitHub server.
///
/// Newtype of Octocrab.
#[derive(Clone)]
pub struct GithubClient2(Octocrab);

#[derive(Debug, Clone, Copy, PartialEq)]
enum PageCursor {
    Page(u8),
    End,
}

impl Default for PageCursor {
    fn default() -> Self {
        Self::Page(0)
    }
}

impl GithubClient2 {
    pub fn new(token: Secret<&str>) -> Result<Self, Error> {
        let client = Octocrab::builder()
            .add_header(
                HeaderName::from_static("user-agent"),
                USER_AGENT.to_string(),
            )
            .personal_token(token.into_inner().to_owned())
            .build()?;
        Ok(Self(client))
    }

    /// Lists current user repositories.
    pub fn list_owned_repositories(&self) -> impl Stream<Item = Result<GhRepository, Error>> + '_ {
        stream::try_unfold(PageCursor::default(), move |cursor| async move {
            // convert page cursor to literal page number
            let page_num = match cursor {
                PageCursor::Page(x) => x,
                PageCursor::End => {
                    // short circuit when page cursor is at the end
                    return Result::<_, Error>::Ok(None);
                }
            };
            // do the thing
            let mut page = self
                .0
                .current()
                .list_repos_for_authenticated_user()
                .affiliation("owner")
                .sort("updated")
                .direction("desc")
                .per_page(100 /* max */)
                .page(page_num)
                .send()
                .await?;
            // take items from response envelope, this will do memswap
            let items = page.take_items();
            // create updated page cursor for the next iteration
            let cursor = if page.next.is_none() {
                // has no next page, must be the end
                PageCursor::End
            } else {
                PageCursor::Page(page_num + 1)
            };
            // yield
            Ok(Some((items, cursor)))
        })
        .map_ok(|x|
            // convert items into its own stream
            stream::iter(x)
            // align inner stream type to the outer stream
            .map(Result::<_, Error>::Ok))
        // flatten the inner stream
        .try_flatten()
    }

    /// Gets the latest commit of a repository.
    pub async fn get_latest_commit(
        &self,
        repo_id: &impl RepoId,
    ) -> Result<Option<GhCommit>, Error> {
        let owner = repo_id.owner();
        let name = repo_id.name();
        let commits: Page<_> = self
            .0
            .get::<_, _, ()>(format!("repos/{owner}/{name}/commits"), None)
            .await?;
        let commit = commits.into_iter().next();
        Ok(commit)
    }

    pub async fn get_check_runs_for_gitref(
        &self,
        repo_id: &impl RepoId,
        gitref: &str,
    ) -> Result<Vec<GhCheckRun>, Error> {
        let owner = repo_id.owner();
        let name = repo_id.name();
        let path = format!("repos/{owner}/{name}/commits/{gitref}/check-runs?per_page=100");
        #[derive(Deserialize)]
        struct Envelope {
            check_runs: Vec<GhCheckRun>,
        }
        let response: Envelope = self.0.get::<_, _, ()>(path, None).await?;
        Ok(response.check_runs)
    }
}
