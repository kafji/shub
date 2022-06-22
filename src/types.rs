//! Defines application domain data types.

use crate::{github_models::GhRepository, repository_id::IsRepositoryId};
use anyhow::bail;
use std::{fmt, str::FromStr};
use thiserror::Error;

// types ------------------------------

#[derive(Debug, PartialEq, Clone)]
pub struct Repository {
    pub name: String,
    pub owner: String,
    pub a_fork: bool,
    pub archived: bool,
    pub build_status: Option<BuildStatus>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum BuildStatus {
    Success,
    Failure,
    InProgress,
}

// end: types ------------------------------

// Repository impls ------------------------------

impl IsRepositoryId for Repository {
    fn owner(&self) -> &str {
        &self.owner
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl TryFrom<GhRepository> for Repository {
    type Error = anyhow::Error;

    fn try_from(x: GhRepository) -> Result<Self, Self::Error> {
        let owner = {
            let owner = x.owner.map(|x| x.login);
            match owner {
                Some(x) => x,
                None => bail!("owner can not be none, was `{:?}`", owner),
            }
        };
        let s = Self {
            name: x.name,
            owner,
            a_fork: x.fork.unwrap_or_default(),
            archived: x.archived.unwrap_or_default(),
            build_status: None,
        };
        Ok(s)
    }
}

// end: Repository impls ------------------------------

// BuildStatus impls ------------------------------

impl fmt::Display for BuildStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BuildStatus::*;
        let s = match self {
            Success => "success",
            Failure => "failure",
            InProgress => "in_progress",
        };
        f.write_str(s)
    }
}

impl FromStr for BuildStatus {
    type Err = ParseBuildStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use BuildStatus::*;
        let s = match s {
            "success" => Success,
            "failure" => Failure,
            "in_progress" => InProgress,
            _ => {
                let err = ParseBuildStatusError(format!("unexpected string, was `{}`", s));
                return Err(err);
            }
        };
        Ok(s)
    }
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct ParseBuildStatusError(String /* message */);

// end: BuildStatus impls ------------------------------
