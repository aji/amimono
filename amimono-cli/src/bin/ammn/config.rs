use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub project: ProjectConfig,
    pub target: HashMap<String, TargetConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "format")]
pub enum ProjectConfig {
    Cargo,
    External { path: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "driver")]
pub enum TargetConfig {
    Kubernetes { cluster: String },
}

pub fn load() -> Config {
    let cf_file = match std::fs::read_to_string("amimono.toml") {
        Ok(x) => x,
        Err(e) => crate::fatal!("failed to load amimono.toml: {}", e),
    };
    match toml::de::from_str(&cf_file) {
        Ok(x) => x,
        Err(e) => crate::fatal!("failed to parse amimono.toml: {}", e),
    }
}
