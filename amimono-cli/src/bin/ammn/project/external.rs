use std::{path::PathBuf, str::FromStr};

use crate::project::Project;

pub struct ExternalProject(pub String);
impl Project for ExternalProject {
    fn build_local(&self) -> PathBuf {
        log::info!("using external project {}", self.0);
        PathBuf::from_str(&self.0).unwrap()
    }
}
