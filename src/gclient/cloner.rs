use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::{fs, path::Path, process::Command};

use anyhow::{bail, Context, Result};
use futures::future::try_join_all;
use itertools::Itertools;
use linya::{Bar, Progress};
use path_absolutize::*;
use pyo3::types::PyBool;
use pyo3::Python;
use smart_default::SmartDefault;
use url::Url;
use zip::ZipArchive;

use crate::cipd::common::GENERIC_HTTP_CLIENT;
use crate::cipd::repository::{get_instance_url, resolve_instance};
use crate::gclient::gn_args::generate_gn_args;
use crate::gclient::var_utils::{set_builtin_vars, set_vars_from_hashmap};
use crate::types::deps::{CacheKVList, Dependency, DependencyDef, DepsSpec};
use crate::types::dotgclient::{Dotgclient, Solution};

use super::entries_cache::{path_to_entries_cache, read_entries};

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

    #[default = false]
    pub cipd_ignore_platformed: bool,
}

#[derive(Clone)]
struct NumberedDependency {
    pub dep_num: usize,
    pub tmp_path: PathBuf,
    pub clone_path: PathBuf,
    pub dependency: Dependency,
    pub required_num: Option<usize>,
}

pub async fn clone_dependencies<P: AsRef<Path>>(
    spec: &DepsSpec,
    base_path_: P,
    solution: &Solution,
    dotgclient: &Dotgclient,
    opts: SyncOptions,
) -> Result<()> {
    let base_path = base_path_.as_ref();

    let mut deps_with_contitions = Python::with_gil(|py| {
        let mut spec_vars = spec.vars.clone();
        if let Some(custom_vars) = solution.custom_vars.clone() {
            spec_vars.extend(custom_vars);
        }
        let (globals, vars) = set_vars_from_hashmap(py, &spec_vars);
        set_builtin_vars(&dotgclient, vars);
        if opts.verbosity >= 2 {
            println!("{}", vars);
        }

        generate_gn_args(&py, globals, vars, spec, base_path);

        let mut deps: Vec<(String, Dependency, CacheKVList)> = vec![];
        for (clone_path, dep_def) in &spec.deps {
            match dep_def {
                DependencyDef::Simple(_) => {
                    let dep: Dependency = dep_def.into();
                    deps.push((
                        clone_path.to_owned(),
                        dep.clone(),
                        dep.to_cache_kv_list(clone_path),
                    ))
                }
                DependencyDef::Normal(dep) => {
                    if opts.cipd_ignore_platformed {
                        if let Dependency::CIPD { packages, .. } = dep {
                            if packages.iter().any(|p| p.package.contains("${{")) {
                                continue;
                            }
                        }
                    }
                    let cache_kv_list = dep.to_cache_kv_list(clone_path);
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
                            deps.push((clone_path.to_owned(), dep.to_owned(), cache_kv_list));
                        }
                    } else {
                        deps.push((clone_path.to_owned(), dep.to_owned(), cache_kv_list));
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

    deps_with_contitions.sort_by_cached_key(|(clone_path, ..)| clone_path.to_owned());

    let entries_cache_path = path_to_entries_cache(base_path);
    let previous_entries_cache = read_entries(entries_cache_path)?;

    let mut new_entries_cache = HashMap::new();

    // fill new cache with dependencies
    for (_, _, cache_kv_list) in &deps_with_contitions {
        for (k, v) in cache_kv_list {
            if new_entries_cache.insert(k.clone(), v).is_some() {
                bail!("duplicate key: {:?}", k);
            }
        }
    }

    // delete the git repositories that were removed from spec
    // and paths with cipd packages that were either removed or changed
    let paths_to_delete = previous_entries_cache
        .keys()
        .filter(|&k| !new_entries_cache.contains_key(k))
        .chain(
            new_entries_cache
                .iter()
                // if key exists in new entries, it's cipd and value (version) changed
                .filter(|&(k, v)| k.contains(':') && previous_entries_cache.get(k) != Some(v))
                .map(|(k, _)| k),
        )
        .map(|k| {
            // if cipd, format is "{path}:{package}". if git, it's just path
            k.split_once(':').map(|(path, _)| path).unwrap_or(k)
        })
        .unique()
        .collect_vec();
    if opts.verbosity >= 2 {
        println!("{} paths to delete", paths_to_delete.len());
    }
    for path_str in paths_to_delete {
        let path_ = PathBuf::try_from(path_str)?;
        let path = path_.absolutize_from(base_path)?;
        if opts.verbosity >= 2 {
            println!("deleting {:?}", path);
        }
        match fs::remove_dir_all(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
            Ok(()) => {}
        }
    }

    let tpot_cipd_path = base_path.join(".tpot_cipd");
    fs::create_dir_all(&tpot_cipd_path).expect("create .tpot_cipd dir");

    // if spec didn't change, we're not updating it
    let deps_to_update = deps_with_contitions
        .into_iter()
        .filter(|(_, _, cache_kv_list)| {
            cache_kv_list
                .iter()
                .any(|(k, v)| previous_entries_cache.get(k) != Some(v))
        });

    let mut dep_num = 0;
    let mut numbered_deps: Vec<NumberedDependency> = deps_to_update
        .into_iter()
        .map(|(clone_path, dep, _)| {
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
                tmp_path: tpot_cipd_path.clone(),
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
            match handle.join().unwrap().await {
                Ok(dep_num) => {
                    done.insert(dep_num);
                    if let Some(bar) = &bar {
                        progress.set_and_draw(bar, done.len());
                    }
                }
                Err(e) => panic!("{:#?}", e),
            }
        }
    }
    Ok(())
}

// pub and out of handle_dep() for handling .gclient solutions
pub fn git_clone<P: AsRef<Path>>(url_spec: &str, clone_path: P, opts: &SyncOptions) -> Result<()> {
    let mut url_parsed = Url::parse(&url_spec).unwrap();
    let url_path = url_parsed.path().to_string();
    let (git_path, git_ref) = if url_path.contains('@') {
        let (p, r) = url_path.split_once('@').unwrap();
        (p, Some(r))
    } else {
        (url_path.as_str(), None)
    };
    url_parsed.set_path(git_path);
    let url = url_parsed.clone().to_string();

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
            clone_path.as_ref(),
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
            clone_path.as_ref(),
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
            clone_path.as_ref(),
            git_merge.status.code(),
            String::from_utf8(git_merge.stderr).unwrap(),
        );
    }
    Ok(())
}

async fn handle_dep(
    NumberedDependency {
        dep_num,
        tmp_path,
        clone_path,
        dependency,
        ..
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
            if opts.verbosity >= 1 {
                println!("cloning {} to {}", url_spec, clone_path.to_str().unwrap());
            }
            git_clone(&url_spec, &clone_path, &opts)
                .with_context(|| format!("while cloning {} to {:?}", url_spec, clone_path))?;
        }
        Dependency::CIPD {
            packages,
            condition: _,
        } => {
            let instances = try_join_all(
                packages
                    .iter()
                    .map(|p| resolve_instance(&p.package, &p.version)),
            )
            .await?;
            for instance in &instances {
                let digest = instance.digest.clone().unwrap();
                let zip_file = tmp_path.join(&format!("{}.zip", &digest.hex_digest));
                let instance_url = get_instance_url(&instance.package, &digest)
                    .await
                    .with_context(|| {
                        format!("getting cipd instance url (clone path: {:?})", clone_path)
                    })?;
                fs::write(
                    &zip_file,
                    GENERIC_HTTP_CLIENT
                        .get(&instance_url)
                        .send()
                        .await
                        .with_context(|| format!("downloading cipd instance: {:?}", instance_url))?
                        .bytes()
                        .await
                        .with_context(|| {
                            format!("getting bytes of cipd instance: {:?}", instance_url)
                        })?,
                )
                .with_context(|| format!("writing cipd zip: {:?}", zip_file))?;
                ZipArchive::new(fs::File::open(&zip_file).expect("reading cipd instance file"))
                    .with_context(|| format!("parsing cipd instance file: {:?}", zip_file))?
                    .extract(&clone_path)
                    .with_context(|| format!("extracting cipd instance to: {:?}", clone_path))?;
                // XXX: .cipdpkg/manifest.json files have permissions set to 400, leading to
                // OS permission errors on override attempt - every time there's
                // more than 1 cipd package in a single directory
                fs::remove_dir_all(clone_path.join(".cipdpkg"))?;
            }
        }
    };
    Ok(dep_num)
}
