pub mod app;

use anyhow::{bail, Error};
use std::str::FromStr;

#[derive(PartialEq, Clone, Debug)]
pub struct RepositoryId {
    pub owner: String,
    pub name: String,
}

impl RepositoryId {
    pub fn new(owner: impl Into<String>, name: impl Into<String>) -> Self {
        let owner = owner.into();
        let name = name.into();
        Self { owner, name }
    }

    pub fn from_partial(
        PartialRepositoryId { owner, name }: PartialRepositoryId,
        default_owner: String,
    ) -> Self {
        Self { owner: owner.unwrap_or(default_owner), name }
    }
}

impl FromStr for RepositoryId {
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
fn test_parse_repository_id() {
    // trivial case
    assert_eq!(
        RepositoryId { owner: "kafji".to_owned().into(), name: "shub".to_owned() },
        "kafji/shub".parse().unwrap()
    );
    // missing owner
    assert_eq!(
        "Expecting in `:owner/:name` format, but was `shub`.",
        "shub".parse::<RepositoryId>().unwrap_err().to_string()
    );
    // missing name
    assert_eq!(
        "Expecting in `:owner/:name` format, but was `kafji/`.",
        "kafji/".parse::<RepositoryId>().unwrap_err().to_string()
    );
    // double separator
    assert_eq!(
        RepositoryId { owner: "kafji".to_owned().into(), name: "sh/ub".to_owned() },
        "kafji/sh/ub".parse().unwrap()
    );
}

#[derive(PartialEq, Clone, Debug)]
pub struct PartialRepositoryId {
    pub owner: Option<String>,
    pub name: String,
}

impl PartialRepositoryId {
    pub fn complete(self, default_owner: impl Into<String>) -> RepositoryId {
        RepositoryId::from_partial(self, default_owner.into())
    }
}

impl FromStr for PartialRepositoryId {
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
            None => Self { owner: None, name: s.into() },
        };
        Ok(r)
    }
}

#[cfg(test)]
#[test]
fn test_parse_partial_repository_id() {
    // trivial case
    assert_eq!(
        PartialRepositoryId { owner: "kafji".to_owned().into(), name: "shub".to_owned() },
        "kafji/shub".parse().unwrap()
    );
    // missing owner
    assert_eq!(
        PartialRepositoryId { owner: None, name: "shub".to_owned() },
        "shub".parse().unwrap()
    );
    // missing name
    assert_eq!(
        "Expecting in `:owner?/:name` format, but was `kafji/`.",
        "kafji/".parse::<PartialRepositoryId>().unwrap_err().to_string()
    );
    // double separator
    assert_eq!(
        PartialRepositoryId { owner: "kafji".to_owned().into(), name: "sh/ub".to_owned() },
        "kafji/sh/ub".parse().unwrap()
    );
}
