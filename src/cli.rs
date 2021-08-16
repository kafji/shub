use argh::FromArgs;
use std::{convert::Infallible, str::FromStr};

pub use self::{actions::*, repos::*};

#[derive(FromArgs, PartialEq, Debug)]
/// Yet another GitHub CLI.
pub struct Cli {
    #[argh(subcommand)]
    pub cmd: Subcommand,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum Subcommand {
    Actions(Actions),
    Repos(Repos),
}

mod actions {
    use super::*;

    #[derive(FromArgs, PartialEq, Debug)]
    #[argh(subcommand, name = "actions")]
    /// GitHub Actions.
    pub struct Actions {
        #[argh(subcommand)]
        pub cmd: ActionsSubcommand,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argh(subcommand)]
    pub enum ActionsSubcommand {
        DeleteRuns(DeleteRuns),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argh(subcommand, name = "delete-runs")]
    /// Delete all workflow runs.
    pub struct DeleteRuns {
        #[argh(positional)]
        pub repository: Repository,
    }
}

mod repos {
    use super::*;
    use std::path::PathBuf;

    #[derive(FromArgs, PartialEq, Debug)]
    #[argh(subcommand, name = "repos")]
    /// GitHub Repositories.
    pub struct Repos {
        #[argh(subcommand)]
        pub cmd: ReposSubcommand,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argh(subcommand)]
    pub enum ReposSubcommand {
        DownloadSettings(DownloadSettings),
        ApplySettings(ApplySettings),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argh(subcommand, name = "download-settings")]
    /// Download GitHub repository settings into a toml file.
    pub struct DownloadSettings {
        #[argh(positional)]
        pub repository: Repository,

        #[argh(positional)]
        /// specify path to download settings to
        pub file: PathBuf,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argh(
        subcommand,
        name = "apply-settings",
        example = "shub repos apply-settings ./gh-repo-settings.toml kafji/shub",
        note = "<repository> takes namespaced repository name e.g. `kafji/shub`."
    )]
    /// Apply GitHub repository settings from a toml file.
    pub struct ApplySettings {
        #[argh(positional)]
        pub file: PathBuf,

        #[argh(positional)]
        pub repository: Repository,

        #[argh(positional, arg_name = "repository")]
        pub repositories: Vec<Repository>,
    }
}

pub fn cmd() -> Cli {
    argh::from_env()
}

#[derive(PartialEq, Debug)]
pub struct Repository {
    pub owner: Option<String>,
    pub name: String,
}

impl FromStr for Repository {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sep = s.find('/');
        let r = match sep {
            Some(x) => {
                let owner = s[..x].to_owned().into();
                let name = s[x + 1..].to_owned();
                Repository { owner, name }
            }
            None => Repository {
                owner: None,
                name: s.into(),
            },
        };
        Ok(r)
    }
}

#[cfg(test)]
#[test]
fn test_parse_repository() {
    // trivial case
    assert_eq!(
        Repository {
            owner: "kafji".to_owned().into(),
            name: "shub".to_owned()
        },
        "kafji/shub".parse().unwrap()
    );
    // missing owner
    assert_eq!(
        Repository {
            owner: None,
            name: "shub".to_owned()
        },
        "shub".parse().unwrap()
    );
    // double separator
    assert_eq!(
        Repository {
            owner: "kafji".to_owned().into(),
            name: "sh/ub".to_owned()
        },
        "kafji/sh/ub".parse().unwrap()
    );
}
