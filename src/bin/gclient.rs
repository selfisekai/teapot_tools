use std::{env::current_dir, fs};

use teapot_tools::cloner::{clone_dependencies, SyncOptions};
use teapot_tools::deps_parser::parse_deps;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,

    #[clap(short, long, parse(from_occurrences), global = true)]
    verbose: i8,

    #[clap(short, long, value_parser, default_value_t = false)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    Sync {
        #[clap(short, long, value_parser)]
        jobs: Option<usize>,

        #[clap(short, long, value_parser, default_value_t = false)]
        force: bool,

        #[clap(short, long = "nohooks", value_parser, default_value_t = false)]
        no_hooks: bool,

        #[clap(
            short = 'p',
            long = "noprehooks",
            value_parser,
            default_value_t = false
        )]
        no_prehooks: bool,

        #[clap(long = "no-history", value_parser, default_value_t = false)]
        /// Clones dependencies without git history
        /// - reduces size and time
        no_history: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let verbosity = if cli.quiet { -1 } else { cli.verbose };

    match cli.command {
        Commands::Sync {
            jobs,
            force: _,
            no_hooks: _,
            no_prehooks: _,
            no_history,
        } => {
            let current_dir = current_dir().expect("current dir");

            let deps_file = fs::read_to_string(current_dir.as_path().join("DEPS").as_path())
                .expect("DEPS file should be in your current working directory");
            let spec = parse_deps(&deps_file).unwrap();

            clone_dependencies(
                &spec,
                current_dir.as_path(),
                SyncOptions {
                    no_history,
                    jobs: jobs.unwrap_or(std::thread::available_parallelism().unwrap().get()),
                    verbosity,
                },
            );
        }
    };
}
