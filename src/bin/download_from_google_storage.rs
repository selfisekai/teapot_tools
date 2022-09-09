use std::{env::current_dir, path::PathBuf};

use anyhow::Context;
use clap::Parser;

use path_absolutize::Absolutize;
use teapot_tools::gs::download::{download, download_from_sha1_file};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(value_parser)]
    target: String,

    #[clap(short, long, value_parser)]
    bucket: String,

    #[clap(short, long, value_parser)]
    output: Option<String>,

    #[clap(short, long = "sha1_file", action)]
    sha1_file: bool,

    #[clap(short, long, action)]
    directory: bool,

    #[clap(short, long, action)]
    recursive: bool,

    #[clap(short = 'n', long = "no_auth", action)]
    /// stub
    no_auth: bool,

    #[clap(short = 'c', long = "no_resume", action)]
    /// stub
    no_resume: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // sanity level 1
    if cli.recursive && !cli.directory {
        panic!("--recursive is to be used with --directory");
    }
    if cli.sha1_file && cli.directory {
        panic!("--sha1_file cannot be used with --directory");
    }

    // sanity level 2
    if !cli.sha1_file && !cli.directory {
        assert_eq!(cli.target.len(), 40, "target must be sha1 hex");
        assert!(
            cli.target.chars().all(|c| c.is_ascii_alphanumeric()),
            "target must be sha1 hex"
        );
    }

    let cwd = current_dir().unwrap();
    if !cli.sha1_file && !cli.directory {
        let file_target = cwd.join(&cli.target);
        download(&cli.bucket, &cli.target, &file_target)
            .await
            .with_context(|| {
                format!(
                    "downloading {} from bucket {} to {:?}",
                    cli.target, cli.bucket, file_target
                )
            })
            .unwrap();
    } else if cli.sha1_file {
        let sha1_file_ = PathBuf::from(&cli.target);
        let sha1_file = sha1_file_.absolutize_from(&cwd).unwrap().to_path_buf();
        download_from_sha1_file(&cli.bucket, sha1_file)
            .await
            .unwrap();
    } else {
        panic!("--directory not supported");
    }
}
