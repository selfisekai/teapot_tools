use std::fs;
use std::path::Path;

use anyhow::Result;
use pyo3::prelude::*;
use pyo3::type_object::PyTypeObject;
use pyo3::types::{PyDict, PyString};

use crate::types::deps::DepsSpec;

pub fn parse_deps(path: &Path) -> Result<DepsSpec> {
    Python::with_gil(|py| -> Result<DepsSpec> {
        let globals = PyDict::new(py);
        // copy builtins (str()) over to globals
        globals
            .set_item("__builtins__", py.eval("__builtins__", None, None).unwrap())
            .unwrap();
        globals
            .set_item("json", py.import("json").unwrap())
            .unwrap();
        globals
            .set_item("Str", py.eval("__builtins__.str", None, None).unwrap())
            .unwrap();
        globals
            .set_item(
                "Var",
                py.eval("lambda x: vars[x]", Some(globals), None).unwrap(),
            )
            .unwrap();

        py.run(
            &fs::read_to_string(path).unwrap(),
            Some(globals),
            Some(globals),
        )
        .unwrap();

        // apparently sometimes they use "{var_name}" and not Var('var_name')
        for (dep_key, dep_val) in globals
            .get_item("deps")
            .unwrap()
            .downcast::<PyDict>()
            .unwrap()
        {
            let key = dep_key.downcast::<PyString>().unwrap();
            if dep_val.is_instance(PyString::type_object(py)).unwrap() {
                py.run(
                    &format!(
                        "deps[{0}] = deps[{0}].format(**vars)",
                        serde_json::to_string(&key.to_string()).unwrap()
                    ),
                    Some(globals),
                    Some(globals),
                )
                .unwrap();
            } else if dep_val.is_instance(PyDict::type_object(py)).unwrap()
                && dep_val
                    .downcast::<PyDict>()
                    .unwrap()
                    .get_item("url")
                    .is_some()
            {
                py.run(
                    &format!(
                        "deps[{0}]['url'] = deps[{0}]['url'].format(**vars)",
                        serde_json::to_string(&key.to_string()).unwrap()
                    ),
                    Some(globals),
                    Some(globals),
                )
                .unwrap();
            }
        }

        // something something "you should convert the Py* types instead of using JSON as intermediate" what about no :chad:
        let result = py
            .eval(
                "json.dumps({'vars': vars, 'deps': deps})",
                Some(globals),
                None,
            )
            .unwrap()
            .downcast::<PyString>()
            .unwrap()
            .to_string();

        Ok(serde_json::from_str::<DepsSpec>(&result).unwrap())
    })
}
