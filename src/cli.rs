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
    Repo {
        #[clap(subcommand)]
        cmd: self::repo::Commands,
    },
    /// Action related operations.
    Action {
        #[clap(subcommand)]
        cmd: self::action::Commands,
    },
    /// Stars related operations.
    Stars {
        #[clap(subcommand)]
        cmd: self::stars::Commands,
    },
    /// Git operations.
    Git {
        #[clap(subcommand)]
        cmd: self::git::Commands,
    },
    /// Workspace operations.
    Ws {
        #[clap(subcommand)]
        cmd: self::ws::Commands,
    },
}

pub mod repo {
    use super::*;

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// Print list of owned repositories.
        Ls {},
        /// Open repository.
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
        /// Fork repository.
        Fork {
            /// Repository identifier.
            repo: RepositoryId,
        },
        /// Clone remote repository.
        Clone {
            /// Repository identifier.
            repo: PartialRepositoryId,
        },
        /// Create repository.
        Create {
            /// Repository identifier.
            repo: PartialRepositoryId,
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

pub mod action {
    use super::*;

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// Print actions status of a repositroy.
        Status {
            /// Repository identifier.
            repo: PartialRepositoryId,
        },
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

pub mod git {
    use super::*;

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// `git commit -am "dump" && git push origin`
        Dump {},
    }
}

pub mod ws {
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
