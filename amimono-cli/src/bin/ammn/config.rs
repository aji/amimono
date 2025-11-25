use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub project: ProjectConfig,
    pub target: HashMap<String, TargetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "format")]
pub enum ProjectConfig {
    Cargo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "driver")]
pub enum TargetConfig {
    Kubernetes {
        context: String,
        image: String,
        env: Option<HashMap<String, String>>,
    },
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
