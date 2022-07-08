use std::collections::HashSet;
use std::path::PathBuf;
use std::{fs, path::Path, process::Command};

use linya::{Bar, Progress};
use path_absolutize::*;
use pyo3::types::PyBool;
use pyo3::Python;
use smart_default::SmartDefault;
use url::Url;

use crate::gn_args::generate_gn_args;
use crate::types::deps::{Dependency, DependencyDef, DepsSpec};
use crate::types::dotgclient::Dotgclient;
use crate::var_utils::{set_builtin_vars, set_vars_from_hashmap};

#[derive(Debug, SmartDefault, Clone)]
pub struct SyncOptions {
    #[default = false]
    pub no_history: bool,

    #[default = 1]
    pub jobs: usize,

    #[default = 1]
    pub git_jobs: usize,

    #[default = 0]
    pub verbosity: i8,
}

#[derive(Clone)]
struct NumberedDependency {
    pub dep_num: usize,
    pub clone_path: PathBuf,
    pub dependency: Dependency,
    pub required_num: Option<usize>,
}

pub fn clone_dependencies(
    spec: &DepsSpec,
    base_path: &Path,
    dotgclient: &Dotgclient,
    opts: SyncOptions,
) {
    let mut deps_with_contitions = Python::with_gil(|py| {
        let (globals, vars) = set_vars_from_hashmap(py, &spec.vars);
        set_builtin_vars(&dotgclient, vars);
        if opts.verbosity >= 2 {
            println!("{}", vars);
        }

        generate_gn_args(&py, globals, vars, spec, base_path);

        let mut deps: Vec<(String, Dependency)> = vec![];
        for (clone_path, dep_def) in &spec.deps {
            match dep_def {
                DependencyDef::Simple(_) => deps.push((clone_path.to_owned(), dep_def.into())),
                DependencyDef::Normal(dep) => {
                    let maybe_condition = match dep {
                        Dependency::Git { url: _, condition } => condition,
                        Dependency::CIPD {
                            packages: _,
                            condition,
                        } => condition,
                    };
                    if let Some(condition) = maybe_condition {
                        if opts.verbosity >= 2 {
                            print!("{}: checking... ", clone_path);
                        }
                        let status = py
                            .eval(&format!("bool({})", condition), Some(globals), Some(vars))
                            .unwrap()
                            .downcast::<PyBool>()
                            .unwrap()
                            .is_true();
                        if opts.verbosity >= 2 {
                            println!("{}", status);
                        }
                        if status == true {
                            deps.push((clone_path.to_owned(), dep.to_owned()));
                        }
                    } else {
                        deps.push((clone_path.to_owned(), dep.to_owned()));
                    }
                }
            }
        }
        deps
    });
    if opts.verbosity >= 0 {
        println!(
            "{} out of {} matching conditions",
            deps_with_contitions.len(),
            spec.deps.len()
        );
    }

    deps_with_contitions.sort_by_cached_key(|(clone_path, _)| clone_path.to_owned());

    let mut dep_num = 0;
    let mut numbered_deps: Vec<NumberedDependency> = deps_with_contitions
        .into_iter()
        .map(|(clone_path, dep)| {
            dep_num += 1;
            let abs_clone_path = base_path
                .join(&clone_path)
                .absolutize()
                .unwrap()
                .to_path_buf();
            if !abs_clone_path.starts_with(base_path) {
                panic!(
                    "{} is outside current workdir (impostor among us)",
                    &clone_path
                );
            }
            NumberedDependency {
                dep_num,
                clone_path: abs_clone_path,
                dependency: dep,
                required_num: None,
            }
        })
        .collect();

    for i in 1..numbered_deps.len() {
        let (a, b) = numbered_deps.split_at_mut(i + 1);

        let i_dep = a.get_mut(i).unwrap();
        for n_dep in b {
            // if i_dep clone_path is inside another dependency, mark it as a requirement
            if i_dep.clone_path.starts_with(&n_dep.clone_path) {
                i_dep.required_num = Some(n_dep.dep_num);
                break;
            }
        }
    }

    let todo_deps = numbered_deps;
    let mut done: HashSet<usize> = HashSet::new();
    let mut progress = Progress::new();
    // if verbosity > 0 the bar is a mess because of the other logs
    let bar: Option<Bar> = if opts.verbosity == 0 {
        Some(progress.bar(todo_deps.len(), "fetching dependencies"))
    } else {
        None
    };
    while todo_deps.len() != done.len() {
        let deps_cur_pass: Vec<_> = todo_deps
            .clone()
            .into_iter()
            .filter(|d| {
                !done.contains(&d.dep_num)
                    && d.required_num.map(|r| done.contains(&r)).unwrap_or(true)
            })
            .take(opts.jobs)
            .map(|dep| {
                let opts_ = opts.clone();
                std::thread::spawn(move || handle_dep(dep, opts_))
            })
            .collect();

        for handle in deps_cur_pass {
            match handle.join().unwrap() {
                Ok(dep_num) => {
                    done.insert(dep_num);
                    if let Some(bar) = &bar {
                        progress.set_and_draw(bar, done.len());
                    }
                }
                Err(e) => panic!("{}", e),
            }
        }
    }
}

// pub and out of handle_dep() for handling .gclient solutions
pub fn git_clone(
    url: &str,
    git_ref: Option<&str>,
    clone_path: PathBuf,
    opts: &SyncOptions,
) -> anyhow::Result<()> {
    // TODO: check if repository exists there in first place
    let git_init = Command::new("git")
        .arg("init")
        // suppresses the warning
        .arg("--initial-branch=master")
        .current_dir(&clone_path)
        .output()
        .expect("git init spawn");
    if git_init.status.code() != Some(0) {
        panic!(
            "git init failed on {:?}, exit code: {:?}\n{}",
            &clone_path,
            git_init.status.code(),
            String::from_utf8(git_init.stderr).unwrap()
        );
    }

    let mut git_fetch_builder = Command::new("git");
    git_fetch_builder.arg("fetch").arg(url);
    if let Some(gref) = git_ref {
        git_fetch_builder.arg(gref);
    }
    if opts.no_history {
        git_fetch_builder.arg("--depth=1");
    }
    git_fetch_builder.arg(format!("--jobs={}", opts.git_jobs));

    let git_fetch = git_fetch_builder
        .current_dir(&clone_path)
        .output()
        .expect("git fetch spawn");
    if git_fetch.status.code() != Some(0) {
        panic!(
            "git fetch failed on {:?}, exit code: {:?}\n{}",
            &clone_path,
            git_fetch.status.code(),
            String::from_utf8(git_fetch.stderr).unwrap(),
        );
    }

    let git_merge = Command::new("git")
        .arg("merge")
        .arg("FETCH_HEAD")
        .current_dir(&clone_path)
        .output()
        .expect("git merge spawn");
    if git_merge.status.code() != Some(0) {
        anyhow::bail!(
            "git merge failed on {:?}, exit code: {:?}\n{}",
            &clone_path,
            git_merge.status.code(),
            String::from_utf8(git_merge.stderr).unwrap(),
        );
    }
    Ok(())
}

fn handle_dep(
    NumberedDependency {
        dep_num,
        clone_path,
        dependency,
        required_num: _,
    }: NumberedDependency,
    opts: SyncOptions,
) -> anyhow::Result<usize> {
    // mkdir -p
    fs::create_dir_all(&clone_path).expect("mkdir success");

    match dependency {
        Dependency::Git {
            url: url_spec,
            condition: _,
        } => {
            let mut url_parsed = Url::parse(&url_spec).unwrap();
            let url_path = url_parsed.path().to_string();
            let (git_path, git_ref) = url_path.split_once('@').unwrap();
            url_parsed.set_path(git_path);
            let url = url_parsed.clone().to_string();
            if opts.verbosity >= 1 {
                println!("cloning {} to {}", url, clone_path.to_str().unwrap());
            }
            git_clone(&url, Some(git_ref), clone_path, &opts).unwrap();
        }
        Dependency::CIPD {
            packages: _,
            condition: _,
        } => {
            if opts.verbosity >= 1 {
                println!("{:?} uses CIPD - unsupported, ignoring", clone_path);
            }
        }
    };
    Ok(dep_num)
}
