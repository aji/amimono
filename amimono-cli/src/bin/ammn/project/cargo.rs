use std::path::PathBuf;

use crate::project::Project;

pub struct CargoProject;

impl Project for CargoProject {
    fn name(&self) -> String {
        let gctx = cargo::GlobalContext::default().unwrap();
        let cwd = std::env::current_dir().unwrap().join("Cargo.toml");
        let ws = cargo::core::Workspace::new(&cwd, &gctx).unwrap();
        ws.current().unwrap().name().to_string()
    }

    fn build_local(&self) -> PathBuf {
        log::info!("building Cargo project");
        let gctx = cargo::GlobalContext::default().unwrap();
        gctx.shell().set_verbosity(cargo::core::Verbosity::Normal);
        let cwd = std::env::current_dir().unwrap().join("Cargo.toml");
        let ws = cargo::core::Workspace::new(&cwd, &gctx).unwrap();
        let options =
            cargo::ops::CompileOptions::new(&gctx, cargo::core::compiler::UserIntent::Build)
                .unwrap();
        let build = match cargo::ops::compile(&ws, &options) {
            Ok(x) => x,
            Err(_) => crate::fatal!("cargo build failed"),
        };
        build.binaries[0].path.clone()
    }
}
