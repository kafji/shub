//! Defines application environment.

use crate::{database::Database, github_client2::GithubClient2};
use anyhow::Error;
use directories_next::BaseDirs;
use std::fs;

/// File system safe application name.
const APP_NAME: &'static str = "shub";

/// The application environment.
pub struct AppEnv<'a> {
    /// Username of current user.
    pub github_username: &'a str,

    /// Github client.
    pub github_client: GithubClient2,

    pub database: Database,
}

impl<'a> AppEnv<'a> {
    /// Creates application environment.
    pub fn new(github_username: &'a str, github_client: GithubClient2) -> Result<Self, Error> {
        let config_dir = BaseDirs::new()
            .map(|x| x.config_dir().to_owned())
            .map(|x| x.join(APP_NAME))
            .expect("failed to get config dir");
        fs::create_dir_all(&config_dir)?;
        let db = {
            let path = config_dir.join("shub.db");
            crate::database::Database::new(&path)?
        };
        Ok(Self {
            github_username,
            github_client,
            database: db,
        })
    }
}
