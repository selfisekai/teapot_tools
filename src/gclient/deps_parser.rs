use anyhow::Result;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};
use pyo3::PyTypeInfo;

use crate::gclient::var_utils::{set_builtin_vars, set_vars_from_hashmap};
use crate::types::deps::DepsSpec;
use crate::types::dotgclient::{Dotgclient, Solution};

pub fn parse_deps(
    deps_file: &String,
    solution: &Solution,
    dotgclient: &Dotgclient,
) -> Result<DepsSpec> {
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
            .set_item(
                "Str",
                py.eval("lambda x: {'literal': str(x)}", None, None)
                    .unwrap(),
            )
            .unwrap();
        let builtin_vars = PyDict::new(py);
        set_builtin_vars(dotgclient, builtin_vars);
        globals
            .set_item("gclient_builtin_vars", builtin_vars)
            .unwrap();
        let custom_vars = PyDict::new(py);
        if let Some(custom_vars) = &solution.custom_vars {
            set_vars_from_hashmap(py, &custom_vars);
        }
        globals
            .set_item("gclient_custom_vars", custom_vars)
            .unwrap();
        py.run(
            include_str!("var_function.py"),
            Some(globals),
            Some(globals),
        )
        .unwrap();

        py.run(deps_file, Some(globals), Some(globals)).unwrap();

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
                "json.dumps(dict((it for it in globals().items() if it[0] in ('vars', 'deps', 'gclient_gn_args', 'gclient_gn_args_file', 'use_relative_paths', 'recursedeps'))))",
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
