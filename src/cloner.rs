use std::{fs, path::Path, process::Command};

use pyo3::type_object::PyTypeObject;
use pyo3::types::PyBool;
use pyo3::types::PyDict;
use pyo3::types::PyString;
use pyo3::Python;

use crate::types::deps::{Dependency, DependencyDef, DepsSpec};

pub fn clone_dependencies(spec: &DepsSpec, base_path: &Path) {
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

    for (rel_clone_path, dep) in &deps_with_contitions {
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
                Command::new("git")
                    .arg("init")
                    .current_dir(&clone_path)
                    .spawn()
                    .expect("git init spawn")
                    .wait()
                    .expect("git init success");

                Command::new("git")
                    .arg("fetch")
                    .arg(url)
                    .arg(&git_ref)
                    .current_dir(&clone_path)
                    .spawn()
                    .expect("git fetch spawn")
                    .wait()
                    .expect("git fetch success");
            }
            Dependency::CIPD {
                packages: _,
                condition: _,
            } => {
                println!("{} uses CIPD - unsupported, ignoring", rel_clone_path);
            }
        }
    }
}
