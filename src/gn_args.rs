use std::fs;
use std::path::Path;
use std::path::PathBuf;

use path_absolutize::Absolutize;
use pyo3::types::PyAny;
use pyo3::types::PyBool;
use pyo3::types::PyDict;
use pyo3::types::PyString;
use pyo3::PyTypeInfo;
use pyo3::Python;

use crate::types::deps::DepsSpec;

fn serialize_gn_arg(py: &Python, globals: &PyDict, vars: &PyDict, item: &PyAny) -> String {
    if item.is_instance(PyDict::type_object(*py)).unwrap() {
        return serde_json::to_string(
            &item
                .get_item("literal")
                .unwrap()
                .downcast::<PyString>()
                .unwrap()
                .to_string(),
        )
        .unwrap();
    } else if item.is_instance(PyBool::type_object(*py)).unwrap() {
        return item.downcast::<PyBool>().unwrap().is_true().to_string();
    } else if item.is_none() {
        return "null".to_string();
    } else if item.hasattr("__bool__").unwrap() {
        return py
            .eval(
                &format!(
                    "bool({})",
                    &item.downcast::<PyString>().unwrap().to_string()
                ),
                Some(globals),
                Some(vars),
            )
            .unwrap()
            .is_true()
            .unwrap()
            .to_string();
    }
    return "null".to_string();
}

fn generate_gn_args_contents(
    py: &Python,
    globals: &PyDict,
    vars: &PyDict,
    spec: &DepsSpec,
) -> String {
    let gclient_gn_args = spec.gclient_gn_args.clone().unwrap();
    let mut lines = vec!["# generated by teapot_tools gclient\n".to_string()];
    for arg in gclient_gn_args {
        lines.push(format!(
            "{} = {}",
            &arg,
            serialize_gn_arg(py, globals, vars, vars.get_item(&arg).unwrap())
        ));
    }
    lines.join("\n") + "\n"
}

pub fn generate_gn_args<P: AsRef<Path>>(
    py: &Python,
    globals: &PyDict,
    vars: &PyDict,
    spec: &DepsSpec,
    base_path: P,
) {
    if spec.gclient_gn_args.is_none() || spec.gclient_gn_args_file.is_none() {
        return ();
    }
    let gn_args_file_ = PathBuf::from(spec.gclient_gn_args_file.as_ref().unwrap());
    let gn_args_file = gn_args_file_.as_path().absolutize().unwrap();
    if !gn_args_file.starts_with(base_path) {
        panic!("gclient_gn_args_file outside base_path (suspicious)");
    }
    fs::create_dir_all(gn_args_file.parent().unwrap()).unwrap();
    fs::write(
        gn_args_file,
        generate_gn_args_contents(py, globals, vars, spec),
    )
    .expect("writing the gclient_gn_args_file");
}
