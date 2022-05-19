use crate::github_models::GhRepository;
use anyhow::{bail, Error};
use core::fmt;
use std::str::FromStr;

#[deprecated]
pub trait GetRepoId {
    fn get_repo_id(&self) -> Result<FullRepoId, Error>;
}

impl GetRepoId for GhRepository {
    fn get_repo_id(&self) -> Result<FullRepoId, Error> {
        let owner = self.owner.as_ref().unwrap().login.clone();
        let name = self.name.clone();
        let id = FullRepoId::new(owner, name);
        Ok(id)
    }
}

#[deprecated]
#[derive(PartialEq, Clone, Debug)]
pub struct FullRepoId {
    pub owner: String,
    pub name: String,
}

impl FullRepoId {
    pub fn new(owner: impl Into<String>, name: impl Into<String>) -> Self {
        let owner = owner.into();
        let name = name.into();
        Self { owner, name }
    }

    pub fn from_partial(
        PartialRepoId { owner, name }: PartialRepoId,
        default_owner: String,
    ) -> Self {
        Self {
            owner: owner.unwrap_or(default_owner),
            name,
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for FullRepoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

impl FromStr for FullRepoId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sep = s.find('/');
        let r = match sep {
            Some(x) => {
                let name = &s[x + 1..];
                if name.is_empty() {
                    bail!("Expecting in `:owner/:name` format, but was `{}`.", s)
                }
                let name = name.to_owned();
                let owner = s[..x].to_owned();
                Self { owner, name }
            }
            None => {
                bail!("Expecting in `:owner/:name` format, but was `{}`.", s)
            }
        };
        Ok(r)
    }
}

#[cfg(test)]
#[test]
fn test_repository_id_display() {
    assert_eq!(FullRepoId::new("kafji", "shub").to_string(), "kafji/shub");
}

#[cfg(test)]
#[test]
fn test_parse_repository_id() {
    // trivial case
    assert_eq!(
        FullRepoId {
            owner: "kafji".to_owned().into(),
            name: "shub".to_owned()
        },
        "kafji/shub".parse().unwrap()
    );
    // missing owner
    assert_eq!(
        "Expecting in `:owner/:name` format, but was `shub`.",
        "shub".parse::<FullRepoId>().unwrap_err().to_string()
    );
    // missing name
    assert_eq!(
        "Expecting in `:owner/:name` format, but was `kafji/`.",
        "kafji/".parse::<FullRepoId>().unwrap_err().to_string()
    );
    // double separator
    assert_eq!(
        FullRepoId {
            owner: "kafji".to_owned().into(),
            name: "sh/ub".to_owned()
        },
        "kafji/sh/ub".parse().unwrap()
    );
}

#[deprecated]
#[derive(PartialEq, Clone, Debug)]
pub struct PartialRepoId {
    pub owner: Option<String>,
    pub name: String,
}

impl PartialRepoId {
    pub fn complete(self, default_owner: impl Into<String>) -> FullRepoId {
        FullRepoId::from_partial(self, default_owner.into())
    }
}

impl FromStr for PartialRepoId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sep = s.find('/');
        let r = match sep {
            Some(x) => {
                let name = &s[x + 1..];
                if name.is_empty() {
                    bail!("Expecting in `:owner?/:name` format, but was `{}`.", s)
                }
                let name = name.to_owned();
                let owner = s[..x].to_owned().into();
                Self { owner, name }
            }
            None => Self {
                owner: None,
                name: s.into(),
            },
        };
        Ok(r)
    }
}

#[cfg(test)]
#[test]
fn test_parse_partial_repository_id() {
    // trivial case
    assert_eq!(
        PartialRepoId {
            owner: "kafji".to_owned().into(),
            name: "shub".to_owned()
        },
        "kafji/shub".parse().unwrap()
    );
    // missing owner
    assert_eq!(
        PartialRepoId {
            owner: None,
            name: "shub".to_owned()
        },
        "shub".parse().unwrap()
    );
    // missing name
    assert_eq!(
        "Expecting in `:owner?/:name` format, but was `kafji/`.",
        "kafji/".parse::<PartialRepoId>().unwrap_err().to_string()
    );
    // double separator
    assert_eq!(
        PartialRepoId {
            owner: "kafji".to_owned().into(),
            name: "sh/ub".to_owned()
        },
        "kafji/sh/ub".parse().unwrap()
    );
}

pub trait RepoId {
    fn owner(&self) -> &str;
    fn name(&self) -> &str;
}

pub trait PartialRepoId2 {
    fn owner(&self) -> Option<&str>;

    fn name(&self) -> &str;

    fn into_full<'a>(&'a self, default_owner: &'a str) -> RepositoryId2 {
        let owner = self.owner().unwrap_or(default_owner);
        let name = self.name();
        RepositoryId2 { owner, name }
    }
}

impl<T> PartialRepoId2 for T
where
    T: RepoId,
{
    fn owner(&self) -> Option<&str> {
        Some(RepoId::owner(self))
    }

    fn name(&self) -> &str {
        RepoId::name(self)
    }
}

#[derive(Debug, PartialEq)]
pub struct RepositoryId2<'a> {
    owner: &'a str,
    name: &'a str,
}

impl RepoId for RepositoryId2<'_> {
    fn owner(&self) -> &str {
        &self.owner
    }

    fn name(&self) -> &str {
        &self.name
    }
}
