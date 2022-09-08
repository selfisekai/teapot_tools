use serde::Deserialize;

use crate::types::machine::{GclientCPU, GclientOS};

#[derive(Deserialize, Debug)]
pub struct Solution {
    pub name: String,
    pub url: String,
    pub managed: Option<bool>,
    pub deps_file: Option<String>,
    #[serde(default)]
    /// do not git checkout, just trust the solution is there and follow the DEPS
    pub tpot_no_checkout: bool,
}

#[derive(Deserialize, Debug)]
pub struct Dotgclient {
    #[serde(default)]
    pub solutions: Vec<Solution>,
    #[serde(default)]
    pub target_os: Vec<GclientOS>,
    #[serde(default)]
    pub target_os_only: bool,
    #[serde(default)]
    pub target_cpu: Vec<GclientCPU>,
    #[serde(default)]
    pub target_cpu_only: bool,
}
