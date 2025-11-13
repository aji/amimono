use std::{path::PathBuf, str::FromStr};

pub trait Project {
    fn build_local(&self) -> PathBuf;
}

pub struct CargoProject;
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

pub struct ExternalProject(pub String);
impl Project for ExternalProject {
    fn build_local(&self) -> PathBuf {
        PathBuf::from_str(&self.0).unwrap()
    }
}
