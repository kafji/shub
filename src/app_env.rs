use crate::github_client2::GithubClient2;
use anyhow::Error;
use console::Term;

#[derive(Clone)]
pub struct AppEnv<'a> {
    github_username: &'a str,
    github_client: GithubClient2,
}

impl<'a> AppEnv<'a> {
    pub fn new(github_username: &'a str, github_client: GithubClient2) -> Result<Self, Error> {
        Ok(Self {
            github_username,
            github_client,
        })
    }

    pub fn github_username(&self) -> &'a str {
        self.github_username
    }

    pub fn github_client(&self) -> &GithubClient2 {
        &self.github_client
    }
}
