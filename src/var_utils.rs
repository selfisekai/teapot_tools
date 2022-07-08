use std::collections::HashMap;

use pyo3::type_object::PyTypeObject;
use pyo3::types::{PyDict, PyString};
use pyo3::Python;

use crate::host::{host_cpu, host_os};
use crate::types::deps::VarsPrimitive;
use crate::types::dotgclient::Dotgclient;
use crate::types::machine::{GclientCPU, GclientOS};

pub fn set_vars_from_hashmap<'a>(
    py: Python<'a>,
    vars: &HashMap<String, VarsPrimitive>,
) -> (&'a PyDict, &'a PyDict) {
    let globals = PyDict::new(py);
    globals
        .set_item("__builtins__", py.eval("__builtins__", None, None).unwrap())
        .unwrap();
    globals
        .set_item("vars", serde_json::to_string(vars).unwrap())
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
    globals.set_item("vars", vars).unwrap();
    for (var_name, var_value) in vars {
        if var_value.is_instance(PyString::type_object(py)).unwrap() {
            vars.set_item(
                var_name,
                py.eval(
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
    (globals, vars)
}

/// sets up checkout_* vars based on .gclient file, and host_{cpu,os}
pub fn set_builtin_vars(dotgclient: &Dotgclient, vars: &PyDict) {
    vars.set_item("host_os", &host_os().to_string()).unwrap();
    for os in [
        GclientOS::Mac,
        GclientOS::Win,
        GclientOS::IOS,
        GclientOS::ChromeOS,
        GclientOS::Fuchsia,
        GclientOS::Android,
    ] {
        vars.set_item(
            format!("checkout_{}", os),
            dotgclient.target_os.contains(&os),
        )
        .unwrap();
        vars.set_item(os.to_string(), os.to_string()).unwrap();
    }
    vars.set_item(
        "checkout_linux",
        dotgclient.target_os.contains(&GclientOS::Unix),
    )
    .unwrap();
    vars.set_item("host_cpu", &host_cpu().to_string()).unwrap();
    for cpu in [
        GclientCPU::Arm,
        GclientCPU::Arm64,
        GclientCPU::X86,
        GclientCPU::Mips,
        GclientCPU::Mips64,
        GclientCPU::Ppc,
        GclientCPU::S390,
        GclientCPU::X64,
    ] {
        vars.set_item(
            format!("checkout_{}", cpu),
            dotgclient.target_cpu.contains(&cpu),
        )
        .unwrap();
        vars.set_item(cpu.to_string(), cpu.to_string()).unwrap();
    }
}
