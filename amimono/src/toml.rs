use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AppConfigToml {
    pub jobs: HashMap<String, JobConfigToml>,
}

#[derive(Serialize, Deserialize)]
pub struct JobConfigToml {
    pub replicas: usize,
    pub components: HashMap<String, ComponentToml>,
}

#[derive(Serialize, Deserialize)]
pub struct ComponentToml {
    pub binding: BindingTypeToml,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum BindingTypeToml {
    None,
    Http,
}

#[derive(Serialize, Deserialize)]
pub struct BindingsToml {
    pub components: HashMap<String, BindingToml>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum BindingToml {
    None,
    Http { internal: String, external: String },
}
