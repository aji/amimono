use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    project: Project,
    target: HashMap<String, Target>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    format: ProjectFormat,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectFormat {
    Cargo,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "driver")]
pub enum Target {
    Local(LocalTarget),
    Kubernetes(KubernetesTarget),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalTarget {
    start_port: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KubernetesTarget {
    cluster: String,
}
