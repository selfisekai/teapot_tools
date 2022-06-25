use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum VarsPrimitive {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

/// The whole DEPS file
#[derive(Deserialize, Debug, Default)]
pub struct DepsSpec {
    pub vars: HashMap<String, VarsPrimitive>,
    pub deps: HashMap<String, DependencyDef>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum DependencyDef {
    Simple(String),
    Normal(Dependency),
}

#[derive(Deserialize, Debug)]
// it either has "cipd" or nothing so only untagged works
#[serde(untagged)]
pub enum Dependency {
    Git {
        url: String,
        condition: Option<String>,
    },
    CIPD {
        packages: Vec<CipdPackage>,
        condition: Option<String>,
    },
}

impl From<DependencyDef> for Dependency {
    fn from(def: DependencyDef) -> Self {
        match def {
            DependencyDef::Simple(url) => Dependency::Git {
                url,
                condition: None,
            },
            DependencyDef::Normal(dep) => dep,
        }
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct CipdPackage {
    pub package: String,
    pub version: String,
}
