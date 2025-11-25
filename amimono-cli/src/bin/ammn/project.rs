use std::process::Command;

use amimono_schemas::DumpConfig;

pub enum Project {
    Cargo,
}

impl Project {
    pub fn from_config(cfg: &crate::config::Config) -> Self {
        match &cfg.project {
            crate::config::ProjectConfig::Cargo => Project::Cargo,
        }
    }

    pub fn get_app_config(&self) -> DumpConfig {
        match self {
            Project::Cargo => {
                log::info!("dumping app config via cargo...");
                let out = Command::new("cargo")
                    .args(["run", "--", "--dump-config"])
                    .stderr(std::process::Stdio::inherit())
                    .output()
                    .unwrap_or_else(|e| crate::fatal!("failed to run cargo: {}", e));
                if !out.status.success() {
                    crate::fatal!(
                        "cargo process exited with status {}",
                        out.status.code().unwrap_or(-1)
                    );
                }
                let s = String::from_utf8(out.stdout)
                    .unwrap_or_else(|e| crate::fatal!("failed to parse cargo output: {}", e));
                serde_json::from_str(&s)
                    .unwrap_or_else(|e| crate::fatal!("failed to parse app config: {}", e))
            }
        }
    }
}
