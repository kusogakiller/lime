use clap::Parser;

#[derive(Parser)]
#[command(
    name = "citrus",
    about = "Lime build tool and package manager",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Create a new Lime project
    New {
        /// Project name
        name: String,
    },
    /// Build the project
    Build {
        /// Build in release mode with optimizations
        #[arg(long)]
        release: bool,
    },
    /// Build and run the project
    Run {
        /// Build in release mode before running
        #[arg(long)]
        release: bool,
        /// Program arguments to forward to the executable
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Discover and run tests
    Test {
        /// Build in release mode before testing
        #[arg(long)]
        release: bool,
    },
    /// Format project source files
    Fmt,
}

pub fn parse() -> Command {
    Cli::parse().command
}
