use std::path::PathBuf;

use crate::config::{Config, ProjectConfig};

pub mod cargo;
pub mod external;

pub trait Project {
    fn name(&self) -> String;
    fn build_local(&self) -> PathBuf;
}

pub fn get(cf: &Config) -> Box<dyn Project> {
    let proj: Box<dyn Project> = match &cf.project {
        ProjectConfig::Cargo => Box::new(cargo::CargoProject),
        ProjectConfig::External { name, path } => Box::new(external::ExternalProject {
            name: name.to_owned(),
            path: path.to_owned(),
        }),
    };
    proj
}

pub fn run_local(proj: &dyn Project) {
    use std::process::{Command, Stdio};

    let bin = proj.build_local();

    log::info!("running project locally");
    let out = Command::new(bin.as_os_str())
        .env("AMIMONO_JOB", "_local")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    match out {
        Ok(x) => {
            if x.success() {
                log::warn!("project exited normally");
            } else {
                crate::fatal!("project exited with status {}", x);
            }
        }
        Err(e) => {
            crate::fatal!("failed to run project: {}", e);
        }
    }
}
