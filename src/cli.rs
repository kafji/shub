use clap::{Parser, Subcommand};
use shub::{PartialRepositoryId, RepositoryId};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Cli {
    #[clap(subcommand)]
    pub cmd: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Repository related operations.
    Repos {
        #[clap(subcommand)]
        cmd: self::repos::Commands,
    },
    /// Alias for repos.
    R {
        #[clap(subcommand)]
        cmd: self::repos::Commands,
    },
    /// Stars related operations.
    Stars {
        #[clap(subcommand)]
        cmd: self::stars::Commands,
    },
    /// Workspace operations.
    Workspace {
        #[clap(subcommand)]
        cmd: self::workspace::Commands,
    },
}

pub mod repos {
    use super::*;

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// Print list of owned repositories.
        Ls {},
        /// Open repository.
        Open {
            /// Repository identifier.
            repo: Option<PartialRepositoryId>,

            /// Open the upstream repository.
            #[clap(long)]
            upstream: bool,
        },
        /// Repository settings operation.
        Settings {
            #[clap(subcommand)]
            cmd: self::settings::Commands,
        },
        /// Fork repository.
        Fork {
            /// Repository identifier.
            repo: RepositoryId,
        },
        /// Clone remote repository. Only support cloning owned repository.
        Clone {
            /// Repository identifier.
            repo: PartialRepositoryId,
        },
        /// Create repository.
        Create {
            /// Repository identifier.
            repo: PartialRepositoryId,
        },
        /// Delete repository. Only support deleting forked repository.
        Delete {
            /// Repository identifier.
            repo: PartialRepositoryId,
        },
        /// Print actions status of a repoistory.
        Status {
            /// Repository identifier.
            repo: Option<PartialRepositoryId>,
        },
    }

    pub mod settings {
        use super::*;

        #[derive(Subcommand, Debug)]
        pub enum Commands {
            /// Print repository settings.
            View {
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

pub mod stars {
    use super::*;

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// Print list of starred repositories.
        Ls {},

        /// Star an unstarred repository.
        Star { repo: RepositoryId },

        /// Unstar a starred repository.
        Unstar { repo: RepositoryId },
    }
}

pub mod workspace {
    use super::*;

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// Print list of projects under specified namespace.
        Ls { namespace: String },

        /// Print list of namespaces.
        Namespaces {},
    }
}

pub fn cmd() -> Cli {
    Cli::parse()
}
