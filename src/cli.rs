use clap::{Parser, Subcommand};
use shub::{PartialRepositoryId, RepositoryId};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Cli {
    #[clap(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Repository related operations.
    Repos {
        #[clap(subcommand)]
        cmd: repos::Command,
    },
    /// Alias for repos.
    R {
        #[clap(subcommand)]
        cmd: repos::Command,
    },
    /// Stars related operations.
    Stars {
        #[clap(subcommand)]
        cmd: stars::Command,
    },
    /// Alias for stars.
    S {
        #[clap(subcommand)]
        cmd: stars::Command,
    },
    /// Tasks operations.
    Tasks {
        #[clap(subcommand)]
        cmd: tasks::Command,
    },
    /// Alias for tasks.
    T {
        #[clap(subcommand)]
        cmd: tasks::Command,
    },
    /// Workspace operations.
    Workspace {
        #[clap(subcommand)]
        cmd: workspace::Command,
    },
}

pub mod repos {
    use super::*;

    #[derive(Subcommand, Debug)]
    pub enum Command {
        /// Browse upstream repository of a fork.
        BrowseUpstream {
            /// Repository identifier.
            repo: Option<PartialRepositoryId>,
        },
        /// Repository settings operation.
        Settings {
            #[clap(subcommand)]
            cmd: settings::Command,
        },
        /// Clone remote repository.
        Clone {
            /// Repository identifier.
            repo: PartialRepositoryId,
        },
        /// Print build status of a repoistory.
        BuildStatus {
            /// Repository identifier.
            repo: Option<PartialRepositoryId>,
        },
    }

    pub mod settings {
        use super::*;

        #[derive(Subcommand, Debug)]
        pub enum Command {
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
    pub enum Command {
        /// Print starred repositories.
        Ls,

        /// Star an unstarred repository.
        Star { repo: RepositoryId },

        /// Unstar a starred repository.
        Unstar { repo: RepositoryId },
    }
}

pub mod tasks {
    use super::*;

    #[derive(Subcommand, Debug)]
    pub enum Command {
        /// Print issues and pull requests assigned to me.
        Ls,
    }
}

pub mod workspace {
    use super::*;

    #[derive(Subcommand, Debug)]
    pub enum Command {
        /// Print local projects.
        Ls,
    }
}

pub fn cmd() -> Cli {
    Cli::parse()
}
