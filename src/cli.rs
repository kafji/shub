use clap::{Parser, Subcommand};
use shub::{PartialRepositoryId, RepositoryId};
use std::{convert::Infallible, str::FromStr};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Cli {
    #[clap(subcommand)]
    pub cmd: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Repository related operations.
    Repo {
        #[clap(subcommand)]
        cmd: self::repo::Commands,
    },
    /// Star related operations.
    Star {
        #[clap(subcommand)]
        cmd: self::star::Commands,
    },
}

pub mod repo {
    use super::*;

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// List owned repositories.
        Ls {},
        /// Open a repository.
        Open {
            /// Repository identifier.
            repo: PartialRepositoryId,

            /// Open the upstream repository.
            #[clap(long)]
            upstream: bool,
        },
        /// Repository settings operation.
        Settings {
            #[clap(subcommand)]
            cmd: self::settings::Commands,
        },
        /// Fork a repository.
        Fork {
            /// Repository identifier.
            repo: RepositoryId,
        },
        /// Clone remote repository.
        Clone {
            /// Repository identifier.
            repo: PartialRepositoryId,
        },
    }

    pub mod settings {
        use super::*;

        #[derive(Subcommand, Debug)]
        pub enum Commands {
            /// Print repository settings.
            Get {
                /// Repository identifier.
                repo: PartialRepositoryId,
            },

            /// Apply repository settings from another repository.
            Apply {
                /// Repository to apply the settings from.
                from: PartialRepositoryId,

                /// Repository to apply the settings to.
                to: PartialRepositoryId,
            },
        }
    }
}

pub mod star {
    use super::*;

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// List starred repositories.
        Ls {},
    }
}

pub fn cmd() -> Cli {
    Cli::parse()
}
