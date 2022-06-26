use std::{env::current_dir, fs};

use teapot_tools::cloner::clone_dependencies;
use teapot_tools::deps_parser::parse_deps;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Sync {
        #[clap(short, long, value_parser, default_value_t = false)]
        force: bool,

        #[clap(short, long, value_parser, default_value_t = false)]
        nohooks: bool,

        #[clap(short = 'p', long, value_parser, default_value_t = false)]
        noprehooks: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Sync {
            force: _,
            nohooks: _,
            noprehooks: _,
        } => {
            let current_dir = current_dir().expect("current dir");

            let deps_file = fs::read_to_string(current_dir.as_path().join("DEPS").as_path())
                .expect("DEPS file should be in your current working directory");
            let spec = parse_deps(&deps_file).unwrap();

            clone_dependencies(&spec, current_dir.as_path());
        }
    };
}
