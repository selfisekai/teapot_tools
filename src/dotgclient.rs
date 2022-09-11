use anyhow::Result;
use pyo3::types::PyDict;
use pyo3::Python;

use crate::host::{gclient_host_cpu, gclient_host_os};
use crate::types::dotgclient::Dotgclient;
use crate::types::machine::{GclientOS, OS_LIST};

pub fn read_dotgclient(contents: String) -> Result<Dotgclient> {
    let result_json = Python::with_gil(|py| {
        let variables = PyDict::new(py);
        let globals = PyDict::new(py);
        globals
            .set_item("__builtins__", py.import("builtins").unwrap())
            .unwrap();
        globals
            .set_item("json", py.import("json").unwrap())
            .unwrap();
        py.run(&contents, Some(variables), Some(variables)).unwrap();
        py.eval(
            "json.dumps(dict((it for it in locals().items() if it[0] in ('solutions', 'target_os', 'target_os_only', 'target_cpu', 'target_cpu_only'))))",
            Some(globals),
            Some(variables),
        )
        .unwrap()
        .to_string()
    });
    let mut result: Dotgclient = serde_json::from_str(&result_json).unwrap();
    if result
        .solutions
        .iter()
        .all(|s| s.tpot_internal_from_recursedeps == true)
    {
        panic!("don't play with me");
    }
    let host_os = gclient_host_os();
    if result.target_os.contains(&GclientOS::All) {
        result.target_os = OS_LIST.into();
    } else if !result.target_os_only && !result.target_os.contains(&host_os) {
        result.target_os.push(host_os);
    }
    let host_cpu = gclient_host_cpu();
    if !result.target_cpu_only && !result.target_cpu.contains(&host_cpu) {
        result.target_cpu.push(host_cpu);
    }
    return Ok(result);
}
