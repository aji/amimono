use std::path::PathBuf;

use crate::project::Project;

pub struct CargoProject;

impl Project for CargoProject {
    fn build_local(&self) -> PathBuf {
        log::info!("building Cargo project");
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
