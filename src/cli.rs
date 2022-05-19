use clap::{Parser, Subcommand};
use crate::repo_id::PartialRepoId;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Cli {
    #[clap(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print dashboard.
    D {
        /// Watch repository build statuses.
        #[clap(long, short('w'))]
        watch: bool,
    },

    /// Repository related operations.
    R {
        #[clap(subcommand)]
        cmd: repos::Command,
    },

    /// Stars related operations.
    S {
        #[clap(subcommand)]
        cmd: stars::Command,
    },

    /// Tasks related operations.
    T {
        #[clap(subcommand)]
        cmd: tasks::Command,
    },

    /// Workspace related operations.
    W {
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
            repo: Option<PartialRepoId>,
        },

        /// Clone remote repository.
        Clone {
            /// Repository identifier.
            repo: PartialRepoId,
        },

        /// Poll build status of a repoistory.
        BuildStatus {
            /// Repository identifier.
            repo: Option<PartialRepoId>,
        },

        /// Print repository settings.
        ViewSettings {
            /// Repository identifier.
            repo: PartialRepoId,
        },

        /// Copy repository settings from another repository.
        CopySettings {
            /// Repository to copy the settings from.
            from: PartialRepoId,

            /// Repository to apply the settings to.
            to: PartialRepoId,
        },
    }
}

pub mod stars {
    use super::*;

    #[derive(Subcommand, Debug)]
    pub enum Command {
        /// Print starred repositories.
        Ls,
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

        /// Open editor to a project.
        Edit {
            /// Project name.
            name: String,
        },

        /// Print path to a project.
        Locate {
            /// Project name.
            name: String,
        },
    }
}

pub fn cli() -> Cli {
    Cli::parse()
}
