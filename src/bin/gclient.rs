use std::collections::HashSet;
use std::{env::current_dir, fs};

use anyhow::Context;
use teapot_tools::gclient::cloner::{clone_dependencies, git_clone, SyncOptions};
use teapot_tools::gclient::deps_parser::parse_deps;

use clap::{Parser, Subcommand};
use teapot_tools::gclient::dotgclient::read_dotgclient;
use teapot_tools::types::dotgclient::Solution;

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

        #[clap(long = "tpot-cipd-ignore-platformed", action)]
        /// Ignore cipd dependencies with host platform variable templates
        /// - pretty surely they are built binaries
        cipd_ignore_platformed: bool,
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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let verbosity = if cli.quiet { -1 } else { cli.verbose };

    match cli.command {
        Commands::Sync {
            jobs: jobs_,
            force: _,
            no_hooks: _,
            no_prehooks: _,
            no_history,
            cipd_ignore_platformed,
        } => {
            let jobs = jobs_.unwrap_or_else(|| std::thread::available_parallelism().unwrap().get());
            let current_dir = current_dir().expect("current dir");

            let dotgclient_location = current_dir.join(cli.gclient_file);
            let dotgclient = read_dotgclient(
                fs::read_to_string(&dotgclient_location)
                    .with_context(|| format!("cannot read file: {:?}", dotgclient_location))
                    .unwrap(),
            )
            .unwrap();

            let mut todo_solutions = dotgclient.solutions.clone();
            let mut done_solutions: HashSet<usize> = HashSet::new();

            while todo_solutions.len() != done_solutions.len() {
                let tbd_solutions: Vec<_> = todo_solutions
                    .clone()
                    .into_iter()
                    .enumerate()
                    .filter(|s| !done_solutions.contains(&s.0))
                    .collect();
                for solution in tbd_solutions.iter().map(|s| &s.1) {
                    let solution_dir = current_dir.join(&solution.name);
                    if !solution.tpot_no_checkout {
                        if verbosity >= 0 {
                            println!("cloning {} ({})", solution.name, solution.url);
                        }
                        fs::create_dir_all(&solution_dir)
                            .with_context(|| {
                                format!("cannot create solution directory: {:?}", &solution_dir)
                            })
                            .unwrap();
                        git_clone(
                            &solution.url,
                            solution_dir.clone(),
                            &SyncOptions {
                                no_history,
                                git_jobs: jobs,
                                verbosity,
                                ..Default::default()
                            },
                        )
                        .unwrap();
                    } else if solution.tpot_internal_from_recursedeps {
                        if verbosity >= 0 {
                            println!("following recursedeps in {}", solution.name);
                        }
                    }

                    let deps_file_location =
                        solution_dir.join(solution.deps_file.as_deref().unwrap_or("DEPS"));
                    let deps_file = fs::read_to_string(&deps_file_location)
                        .with_context(|| format!("cannot read file: {:?}", &deps_file_location))
                        .unwrap();
                    let spec = parse_deps(&deps_file, &solution, &dotgclient).unwrap();

                    // if there are recursedeps, add them to the todo
                    todo_solutions.extend(spec.recursedeps.iter().map(|d| Solution {
                        name: if spec.use_relative_paths {
                            // if paths are relative to current DEPS,
                            // add the path of current DEPS to it
                            format!("{}/{}", solution.name, d)
                        } else {
                            d.clone()
                        },
                        url: "".to_string(),
                        tpot_no_checkout: true,
                        tpot_internal_from_recursedeps: true,
                        ..Default::default()
                    }));

                    clone_dependencies(
                        &spec,
                        if spec.use_relative_paths {
                            current_dir.join(deps_file_location.parent().unwrap())
                        } else {
                            current_dir.clone()
                        },
                        &solution,
                        &dotgclient,
                        SyncOptions {
                            no_history,
                            jobs,
                            verbosity,
                            cipd_ignore_platformed,
                            ..Default::default()
                        },
                    )
                    .await;
                }
                done_solutions.extend(tbd_solutions.iter().map(|s| s.0));
            }
        }
        Commands::Config { spec: maybe_spec } => {
            let dotgclient_location = current_dir().unwrap().join(cli.gclient_file);
            if let Some(spec) = maybe_spec {
                fs::write(&dotgclient_location, spec).expect("saving .gclient");
            } else {
                // display (out of original gclient spec, but fuck it)
                let dotgclient = read_dotgclient(
                    fs::read_to_string(&dotgclient_location)
                        .with_context(|| format!("cannot read file: {:?}", &dotgclient_location))
                        .unwrap(),
                )
                .expect("parsing .gclient");
                println!("{:#?}", dotgclient);
            }
        }
    };
}
