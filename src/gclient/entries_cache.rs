use std::collections::HashMap;
use std::fs::{read_to_string, write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use pyo3::types::{PyDict, PyString};
use pyo3::Python;

pub fn path_to_entries_cache<P: AsRef<Path>>(root_path: P) -> PathBuf {
    root_path.as_ref().join(".gclient_entries")
}

/// key - '{path}', or '{path}:{package}' if cipd.
///
/// value - '{url}@{revision pointer}', url to git or 'https://chrome-infra-packages.appspot.com/{package}'.
/// no revision is also possible.
pub type EntriesCache = HashMap<String, String>;

pub fn read_entries<P: AsRef<Path>>(entries_path: P) -> Result<EntriesCache> {
    let cache_path = entries_path.as_ref();

    // no .gclient_entries is valid. pretend there's no keys and values.
    if !cache_path.exists() {
        return Ok(HashMap::default());
    }

    Python::with_gil(|py| -> Result<EntriesCache> {
        let globals = PyDict::new(py);
        globals.set_item("json", py.import("json")?)?;
        py.run(&read_to_string(cache_path)?, Some(globals), Some(globals))?;
        let result = py
            .eval("json.dumps(entries)", Some(globals), Some(globals))
            .unwrap()
            .downcast::<PyString>()
            .unwrap()
            .to_string();
        Ok(serde_json::from_str(&result)?)
    })
}

pub fn write_entries<P: AsRef<Path>>(entries_path: P, entries_cache: &EntriesCache) -> Result<()> {
    write(
        entries_path,
        format!("entries = {}", serde_json::to_string_pretty(entries_cache)?),
    )?;

    Ok(())
}
