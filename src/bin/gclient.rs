use std::{env::current_dir, fs};

use teapot_tools::cloner::{clone_dependencies, git_clone, SyncOptions};
use teapot_tools::deps_parser::parse_deps;

use clap::{Parser, Subcommand};
use teapot_tools::dotgclient::read_dotgclient;

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

    #[clap(long = "gclientfile", value_parser, default_value = ".gclient")]
    gclient_file: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Download dependencies
    Sync {
        #[clap(short, long, value_parser)]
        /// Amount of concurrent dependency download (git clone) jobs
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
    // gclient config --spec 'solutions = [
    //   {
    //     "name": "src",
    //     "url": "https://chromium.googlesource.com/chromium/src.git",
    //     "managed": False,
    //     "custom_deps": {},
    //     "custom_vars": {},
    //   },
    // ]
    // '
    Config {
        #[clap(long)]
        spec: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let verbosity = if cli.quiet { -1 } else { cli.verbose };

    match cli.command {
        Commands::Sync {
            jobs: jobs_,
            force: _,
            no_hooks: _,
            no_prehooks: _,
            no_history,
        } => {
            let jobs = jobs_.unwrap_or_else(|| std::thread::available_parallelism().unwrap().get());
            let current_dir = current_dir().expect("current dir");

            let dotgclient_location = current_dir.join(cli.gclient_file);
            let dotgclient =
                read_dotgclient(fs::read_to_string(dotgclient_location).unwrap()).unwrap();

            for solution in &dotgclient.solutions {
                let solution_dir = current_dir.join(&solution.name);
                if verbosity >= 0 {
                    println!("cloning {} ({})", solution.name, solution.url);
                }
                fs::create_dir_all(&solution_dir).unwrap();
                git_clone(
                    &solution.url,
                    None,
                    solution_dir.clone(),
                    &SyncOptions {
                        no_history,
                        git_jobs: jobs,
                        verbosity,
                        ..Default::default()
                    },
                )
                .unwrap();

                let deps_file = fs::read_to_string(solution_dir.join("DEPS").as_path())
                    .expect("DEPS file should be in your current working directory");
                let spec = parse_deps(&deps_file).unwrap();

                clone_dependencies(
                    &spec,
                    solution_dir.as_path(),
                    &dotgclient,
                    SyncOptions {
                        no_history,
                        jobs,
                        verbosity,
                        ..Default::default()
                    },
                );
            }
        }
        Commands::Config { spec: maybe_spec } => {
            let dotgclient_location = current_dir().unwrap().join(cli.gclient_file);
            if let Some(spec) = maybe_spec {
                fs::write(&dotgclient_location, spec).expect("saving .gclient");
            } else {
                // display (out of original gclient spec, but fuck it)
                let dotgclient = read_dotgclient(
                    fs::read_to_string(&dotgclient_location).expect("reading .gclient"),
                )
                .expect("parsing .gclient");
                println!("{:#?}", dotgclient);
            }
        }
    };
}
