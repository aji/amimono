use std::{
    path::PathBuf,
    process::{Command, Stdio},
    str::FromStr,
};

use crate::config::{Config, ProjectFormat};

pub mod cli;
pub mod config;

fn main() {
    let matches = cli::cli().get_matches();

    match matches.get_one::<String>("project") {
        Some(x) => std::env::set_current_dir(x).expect("could not find project directory"),
        None => (),
    }

    let cf_file =
        std::fs::read_to_string("amimono.toml").expect("no amimono.toml in current directory");
    let cf: Config = toml::de::from_str(&cf_file).expect("could not parse amimono.toml");

    match matches.subcommand() {
        Some(("run", _)) => run_local(&cf),
        Some(("deploy", _)) => unimplemented!(),
        _ => unreachable!(),
    }
}

trait Project {
    fn build_local(&self) -> PathBuf;
}

fn run_local(cf: &Config) {
    match cf.project {
        ProjectFormat::Cargo => run_local_in(&cf, CargoProject),
        ProjectFormat::External { ref path } => run_local_in(&cf, ExternalProject(path.as_str())),
    }
}

fn run_local_in<P: Project>(_cf: &Config, proj: P) {
    let bin = proj.build_local();
    Command::new(bin.as_os_str())
        .env("AMIMONO_JOB", "_local")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("local failed");
}

struct CargoProject;
impl Project for CargoProject {
    fn build_local(&self) -> PathBuf {
        let gctx = cargo::GlobalContext::default().unwrap();
        gctx.shell().set_verbosity(cargo::core::Verbosity::Normal);
        let cwd = std::env::current_dir().unwrap().join("Cargo.toml");
        let ws = cargo::core::Workspace::new(&cwd, &gctx).unwrap();
        let options =
            cargo::ops::CompileOptions::new(&gctx, cargo::core::compiler::UserIntent::Build)
                .unwrap();
        let build = cargo::ops::compile(&ws, &options).unwrap();
        build.binaries[0].path.clone()
    }
}

struct ExternalProject<'a>(&'a str);
impl<'a> Project for ExternalProject<'a> {
    fn build_local(&self) -> PathBuf {
        PathBuf::from_str(self.0).unwrap()
    }
}
