use std::process::{Command, Stdio};

use crate::{
    config::{Config, ProjectConfig, TargetConfig},
    project::{CargoProject, ExternalProject, Project},
    target::{KubernetesTarget, Target},
};

pub mod cli;
pub mod config;
pub mod project;
pub mod target;

fn main() {
    let matches = cli::cli().get_matches();

    match matches.get_one::<String>("project") {
        Some(x) => std::env::set_current_dir(x).expect("could not find project directory"),
        None => (),
    }

    let cf_file =
        std::fs::read_to_string("amimono.toml").expect("no amimono.toml in current directory");
    let cf: Config = toml::de::from_str(&cf_file).expect("could not parse amimono.toml");

    let proj: Box<dyn Project> = match &cf.project {
        ProjectConfig::Cargo => Box::new(CargoProject),
        ProjectConfig::External { path } => Box::new(ExternalProject(path.clone())),
    };

    match matches.subcommand() {
        Some(("run", _)) => do_run(&*proj),
        Some(("deploy", sub)) => {
            let tgt_id = sub.get_one::<String>("target").unwrap();
            let tgt = match cf.target.get(tgt_id).unwrap() {
                TargetConfig::Kubernetes { cluster } => KubernetesTarget(cluster.clone()),
            };
            tgt.deploy(&*proj);
        }
        _ => unreachable!(),
    }
}

fn do_run(proj: &dyn Project) {
    let bin = proj.build_local();
    Command::new(bin.as_os_str())
        .env("AMIMONO_JOB", "_local")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("local failed");
}
