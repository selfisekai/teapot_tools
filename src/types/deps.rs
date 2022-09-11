use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum VarsPrimitive {
    String(String),
    LiteralString(LiteralString),
    Int(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LiteralString {
    literal: String,
}

/// The whole DEPS file
#[derive(Deserialize, Debug, Default, Clone)]
pub struct DepsSpec {
    pub vars: HashMap<String, VarsPrimitive>,
    pub deps: HashMap<String, DependencyDef>,
    pub gclient_gn_args_file: Option<String>,
    pub gclient_gn_args: Option<Vec<String>>,
    #[serde(default)]
    pub use_relative_paths: bool,
    #[serde(default)]
    pub recursedeps: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum DependencyDef {
    Simple(String),
    Normal(Dependency),
}

#[derive(Deserialize, Debug, Clone)]
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
        Dependency::from(&def)
    }
}

impl From<&DependencyDef> for Dependency {
    fn from(def: &DependencyDef) -> Self {
        match def {
            DependencyDef::Simple(url) => Dependency::Git {
                url: url.to_owned(),
                condition: None,
            },
            DependencyDef::Normal(dep) => dep.to_owned(),
        }
    }
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct CipdPackage {
    pub package: String,
    pub version: String,
}
