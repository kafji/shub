use self::{actions::*, repos::*};
use super::{
    error::Error,
    requests::UpdateRepository,
    responses::{ActionsRuns, Repository, WorkflowRun},
};
use futures::{future, stream, StreamExt, TryStream};
use http::{
    header::{AUTHORIZATION, USER_AGENT},
    HeaderMap, HeaderValue,
};
use lembaran::{
    stream::pagination,
    web_linking::{self, Link, Param},
};
use reqwest::{Client, ClientBuilder};
use std::{convert::TryInto, result::Result};
use tracing::{debug, info};
use url::Url;

type ClientResult<T> = Result<T, Error>;

/// [GitHub REST authentication methods](https://docs.github.com/en/rest/overview/other-authentication-methods).
///
/// [HTTP authorization on MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Authorization).
///
pub trait Authentication {
    /// Encode authentication into HTTP authorization header.
    fn to_authz_value(&self) -> String;
}

#[derive(Debug)]
pub struct GhClient {
    base_url: Url,
    http: Client,
}

impl GhClient {
    pub fn new<'a>(
        base_url: impl Into<Option<Url>>,
        token: &impl Authentication,
    ) -> ClientResult<Self> {
        let base_url: Url = base_url
            .into()
            .map(Result::Ok)
            .unwrap_or_else(|| "https://api.github.com/".parse())?;

        let headers = {
            let mut headers = HeaderMap::new();

            let user_agent = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            headers.insert(USER_AGENT, HeaderValue::from_str(&user_agent)?);

            let authorization = token.to_authz_value();
            headers.insert(AUTHORIZATION, authorization.try_into()?);

            headers
        };

        let http = ClientBuilder::new().default_headers(headers).build()?;

        let client = GhClient { base_url, http };
        debug!(?client);

        Ok(client)
    }

    fn build_url(&self, path: &str) -> Url {
        let mut url = self.base_url.clone();
        url.set_path(path);
        url
    }

    pub fn actions(&self) -> GhActions<'_> {
        GhActions { client: self }
    }

    pub fn repos(&self) -> GhRepos<'_> {
        GhRepos { client: self }
    }
}

mod actions {
    use super::*;

    #[derive(Debug)]
    pub struct GhActions<'c> {
        pub client: &'c GhClient,
    }

    impl GhActions<'_> {
        /// List workflow runs for a repository.
        ///
        /// [GitHub Docs].
        ///
        /// [GitHub Docs]: https://docs.github.com/en/rest/reference/actions#list-workflow-runs-for-a-repository
        pub fn list_workflow_runs<'a>(
            &'a self,
            owner: &'a str,
            repo: &'a str,
        ) -> impl TryStream<Ok = WorkflowRun, Error = Error> + 'a {
            pagination::with_factory(move |url: Option<Url>| async move {
                let url = match url {
                    Some(x) => x,
                    None => self.client.build_url(&format!(
                        "/repos/{owner}/{repo}/actions/runs",
                        owner = owner,
                        repo = repo
                    )),
                };
                let request = self.client.http.get(url).query(&[("per_page", "100")]);
                debug!(?request, "sending request");
                let response = request.send().await?;
                debug!(?response, "received response");
                let response = response.error_for_status()?;
                let next_page_url = web_linking::http::from_headers(response.headers())
                    .find(|Link { params, .. }| {
                        params
                            .iter()
                            .find(|Param { name, value }| {
                                *name == "rel" && value.as_deref() == Some("next".into())
                            })
                            .is_some()
                    })
                    .map(|Link { uri, .. }| uri)
                    .map(|x| String::from_utf8_lossy(&**x).parse::<Url>())
                    .transpose()?;
                let response_body: ActionsRuns = response.json().await?;
                debug!(?response_body, "response body");
                Ok((response_body.workflow_runs, next_page_url))
            })
            .flat_map(|x: ClientResult<Vec<_>>| match x {
                Ok(x) => stream::iter(x).map(|x| Ok(x)).boxed(),
                Err(x) => stream::once(future::ready(Err(x))).boxed(),
            })
        }

        /// Delete a workflow run.
        ///
        /// [GitHub Docs].
        ///
        /// [GitHub Docs]: https://docs.github.com/en/rest/reference/actions#delete-a-workflow-run
        pub async fn delete_workflow_run(
            &self,
            owner: &str,
            repo: &str,
            run_id: i32,
        ) -> ClientResult<()> {
            let url = self.client.build_url(&format!(
                "/repos/{owner}/{repo}/actions/runs/{run_id}",
                owner = owner,
                repo = repo,
                run_id = run_id
            ));
            let http = &self.client.http;
            let request = http.delete(url);
            debug!(?http, ?request, "sending request");
            let response = request.send().await?;
            debug!(?response, "received response");
            response.error_for_status()?;
            Ok(())
        }
    }
}

mod repos {
    use super::*;

    #[derive(Debug)]
    pub struct GhRepos<'c> {
        pub client: &'c GhClient,
    }

    impl GhRepos<'_> {
        /// Get a repository.
        ///
        /// [GitHub Docs].
        ///
        /// [GitHub Docs]: https://docs.github.com/en/rest/reference/repos#get-a-repository
        pub async fn get_repository(&self, owner: &str, repo: &str) -> ClientResult<Repository> {
            let url = self.client.build_url(&format!(
                "/repos/{owner}/{repo}",
                owner = owner,
                repo = repo
            ));
            let request = self.client.http.get(url);
            info!(?request, "sending request");
            let response = request.send().await?;
            info!(?response, "received response");
            let response = response.error_for_status()?;
            let response_body: Repository = response.json().await?;
            debug!(?response_body, "response body");
            Ok(response_body)
        }

        /// Update a repository.
        ///
        /// [GitHub Docs].
        ///
        /// [GitHub Docs]: https://docs.github.com/en/rest/reference/repos#update-a-repository
        pub async fn update_repository(
            &self,
            owner: &str,
            repo: &str,
            fields: &UpdateRepository,
        ) -> ClientResult<()> {
            let url = self.client.build_url(&format!(
                "/repos/{owner}/{repo}",
                owner = owner,
                repo = repo
            ));
            let request = self.client.http.patch(url).json(&fields);
            info!(?request, "sending request");
            let response = request.send().await?;
            info!(?response, "received response");
            response.error_for_status()?;
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::PersonalAccessToken;
        use warp::Filter;

        const TEST_TOKEN: PersonalAccessToken<'static> = PersonalAccessToken::new("kafji", "t0k3n");

        #[tokio::test]
        async fn test_update_repository() {
            let (tx_ready, rx_ready) = tokio::sync::oneshot::channel();

            let server = tokio::spawn(async move {
                // PATCH /repos/kafji/shub
                let route = warp::patch()
                    .and(warp::path!("repos" / "kafji" / "shub"))
                    .and(warp::body::json())
                    .and_then(|body: UpdateRepository| async move {
                        assert_eq!(
                            body,
                            UpdateRepository {
                                allow_merge_commit: false.into(),
                                ..Default::default()
                            }
                        );
                        Result::<_, warp::Rejection>::Ok(warp::reply())
                    });
                let (addr, server) = warp::serve(route).bind_ephemeral(([127, 0, 0, 1], 0));
                let server = tokio::spawn(async move { server.await });
                tx_ready.send(addr).unwrap();
                server.await.unwrap();
            });

            let addr = rx_ready.await.unwrap();
            let base_url: Url = format!("http://{}/", addr).parse().unwrap();
            let client = GhClient::new(base_url, &TEST_TOKEN).unwrap();
            let fields = UpdateRepository {
                allow_merge_commit: false.into(),
                ..Default::default()
            };
            client
                .repos()
                .update_repository("kafji", "shub", &fields)
                .await
                .unwrap();

            server.abort();
            server.await.ok();
        }
    }
}
