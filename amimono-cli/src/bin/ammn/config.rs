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
