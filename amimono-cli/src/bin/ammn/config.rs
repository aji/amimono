use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub project: ProjectFormat,
    pub target: HashMap<String, Target>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "format")]
pub enum ProjectFormat {
    Cargo,
    External { path: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "driver")]
pub enum Target {
    Local(LocalTarget),
    Kubernetes(KubernetesTarget),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalTarget {}

#[derive(Debug, Serialize, Deserialize)]
pub struct KubernetesTarget {
    pub cluster: String,
}
