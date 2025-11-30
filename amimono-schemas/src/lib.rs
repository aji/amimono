use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DumpConfig {
    pub revision: String,
    pub jobs: HashMap<String, DumpJob>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DumpJob {
    pub is_stateful: bool,
    pub components: HashMap<String, DumpComponent>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DumpComponent {
    pub is_stateful: bool,
    pub binding: DumpBinding,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DumpBinding {
    None,
    Rpc,
    Tcp { port: u16 },
}
