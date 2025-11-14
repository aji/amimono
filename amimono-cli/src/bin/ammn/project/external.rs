use std::{path::PathBuf, str::FromStr};

use crate::project::Project;

pub struct ExternalProject {
    pub name: String,
    pub path: String,
}

impl Project for ExternalProject {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn build_local(&self) -> PathBuf {
        log::info!("using external project {}", self.path);
        PathBuf::from_str(&self.path).unwrap()
    }
}
