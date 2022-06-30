use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Stdio;
use std::{fs, path::Path, process::Command};

use pyo3::type_object::PyTypeObject;
use pyo3::types::PyBool;
use pyo3::types::PyDict;
use pyo3::types::PyString;
use pyo3::Python;
use rayon::prelude::*;
use smart_default::SmartDefault;

use crate::types::deps::{Dependency, DependencyDef, DepsSpec};

#[derive(Debug, SmartDefault)]
pub struct SyncOptions {
    #[default = false]
    pub no_history: bool,

    #[default = 1]
    pub jobs: usize,
}

pub fn clone_dependencies(spec: &DepsSpec, base_path: &Path, opts: SyncOptions) {
    let mut deps_with_contitions = Python::with_gil(|py| {
        let globals = PyDict::new(py);
        globals
            .set_item("__builtins__", py.eval("__builtins__", None, None).unwrap())
            .unwrap();
        globals
            .set_item("vars", serde_json::to_string(&spec.vars).unwrap())
            .unwrap();
        globals
            .set_item("json", py.import("json").unwrap())
            .unwrap();
        py.run(
            include_str!("str_to_bool_eval.py"),
            Some(globals),
            Some(globals),
        )
        .unwrap();
        let vars = py
            .eval("json.loads(vars)", Some(globals), None)
            .unwrap()
            .downcast::<PyDict>()
            .unwrap();
        for (var_name, var_value) in vars {
            if var_value.is_instance(PyString::type_object(py)).unwrap() {
                vars.set_item(
                    var_name,
                    py.run(
                        &format!(
                            "str({})",
                            serde_json::to_string(
                                &var_value.downcast::<PyString>().unwrap().to_string()
                            )
                            .unwrap()
                        ),
                        Some(globals),
                        Some(vars),
                    )
                    .unwrap(),
                )
                .unwrap();
            }
        }
        let host_os = if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "macos") {
            "mac"
        } else if cfg!(windows) {
            "win"
        } else {
            panic!("unknown target_os");
        };
        vars.set_item("host_os", host_os).unwrap();
        for os in [
            "linux", "mac", "win", "ios", "chromeos", "fuchsia", "android",
        ] {
            vars.set_item(format!("checkout_{}", os), os == host_os)
                .unwrap();
            vars.set_item(os, os).unwrap();
        }
        let host_cpu = if cfg!(target_arch = "x86_64") {
            "x64"
        } else if cfg!(target_arch = "x86") {
            "ia32"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            // TODO: add more; chromium arch reference: https://nodejs.org/dist/latest-v16.x/docs/api/os.html#osarch
            panic!("unknown target_arch");
        };
        vars.set_item("host_cpu", host_cpu).unwrap();
        for cpu in [
            "arm", "arm64", "x86", "mips", "mips64", "ppc", "s390", "x64",
        ] {
            vars.set_item(format!("checkout_{}", cpu), cpu == host_cpu)
                .unwrap();
            vars.set_item(cpu, cpu).unwrap();
        }
        println!("{}", vars);
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
                        print!("{}: checking... ", clone_path);
                        let status = py
                            .eval(&format!("bool({})", condition), Some(globals), Some(vars))
                            .unwrap()
                            .downcast::<PyBool>()
                            .unwrap()
                            .is_true();
                        println!("{}", status);
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
    println!(
        "{} out of {} matching conditions",
        deps_with_contitions.len(),
        spec.deps.len()
    );

    deps_with_contitions.sort_by_cached_key(|(p, _)| p.to_owned());

    let mut dep_num = 0;
    let mut numbered_deps: Vec<(usize, PathBuf, Dependency, Option<usize>)> = deps_with_contitions
        .into_iter()
        .map(|(clone_path, dep)| {
            dep_num += 1;
            (dep_num, PathBuf::from(clone_path), dep, None)
        })
        .collect();

    for i in 1..numbered_deps.len() {
        let (a, b) = numbered_deps.split_at_mut(i + 1);

        let i_dep = a.get_mut(i).unwrap();
        for n_dep in b {
            // if i_dep clone_path is inside another dependency, mark it as a requirement
            if i_dep.1.starts_with(&n_dep.1) {
                i_dep.3 = Some(n_dep.0);
                break;
            }
        }
    }

    let todo_deps = numbered_deps;
    let mut done: HashSet<usize> = HashSet::new();

    while !todo_deps.is_empty() {
        let deps_that_can_be_done_in_current_pass: Vec<_> = todo_deps
            .iter()
            .filter(|d| d.3.map(|dep| done.contains(&dep)).unwrap_or(true))
            .collect();

        deps_that_can_be_done_in_current_pass
            .par_iter()
            .panic_fuse()
            .for_each(|(_, rel_clone_path, dep, _)| {
                let clone_path = Path::new(base_path).join(rel_clone_path);
                // mkdir -p
                fs::create_dir_all(&clone_path).expect("mkdir success");

                match dep {
                    Dependency::Git {
                        url: url_spec,
                        condition: _,
                    } => {
                        // TODO: use an actual url parser to only split in path part
                        let mut url_split = url_spec.splitn(2, '@').collect::<Vec<&str>>();
                        let git_ref = url_split.pop().unwrap();
                        let url = url_split.pop().unwrap();
                        println!("cloning {} to {}", url, clone_path.to_str().unwrap());
                        // TODO: check if repository exists there in first place
                        let git_init = Command::new("git")
                            .arg("init")
                            // suppresses the warning
                            .arg("--initial-branch=master")
                            .current_dir(&clone_path)
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn()
                            .expect("git init spawn")
                            .wait()
                            .expect("git init wait");
                        if git_init.code() != Some(0) {
                            panic!(
                                "git init failed on {:?}, exit code: {:?}",
                                &clone_path,
                                git_init.code()
                            );
                        }

                        let git_fetch = Command::new("git")
                            .arg("fetch")
                            .arg(url)
                            .arg(&git_ref)
                            .args(if opts.no_history {
                                &["--depth=1"][..]
                            } else {
                                &[][..]
                            })
                            .current_dir(&clone_path)
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn()
                            .expect("git fetch spawn")
                            .wait()
                            .expect("git fetch wait");
                        if git_fetch.code() != Some(0) {
                            panic!(
                                "git fetch failed on {:?}, exit code: {:?}",
                                &clone_path,
                                git_fetch.code()
                            );
                        }

                        let git_merge = Command::new("git")
                            .arg("merge")
                            .arg("FETCH_HEAD")
                            .current_dir(&clone_path)
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn()
                            .expect("git merge spawn")
                            .wait()
                            .expect("git merge wait");
                        if git_merge.code() != Some(0) {
                            panic!(
                                "git merge failed on {:?}, exit code: {:?}",
                                &clone_path,
                                git_merge.code()
                            );
                        }
                    }
                    Dependency::CIPD {
                        packages: _,
                        condition: _,
                    } => {
                        println!("{:?} uses CIPD - unsupported, ignoring", rel_clone_path);
                    }
                }
            });
        done.extend(
            deps_that_can_be_done_in_current_pass
                .into_iter()
                .map(|(dep_num, ..)| dep_num),
        );
    }
}
