use std::{
    error::Error,
    io::Read,
    process::{Command, Stdio},
};

use crate::project::Project;

pub fn go(proj: &dyn Project) {
    let cli = match DockerCli::new() {
        Ok(x) => x,
        Err(e) => crate::fatal!("could not create Docker CLI: {}", e),
    };

    let path = proj.build_local();
    log::info!("{:?}", path.to_str());
}

struct DockerCli {
    version: String,
}

impl DockerCli {
    fn new() -> Result<DockerCli, Box<dyn Error>> {
        let mut cli = DockerCli {
            version: "(unknown)".to_string(),
        };
        let out = cli
            .command()
            .stderr(Stdio::inherit())
            .arg("--version")
            .output()?;
        if !out.status.success() {
            return Err(format!("docker --version failed with status {}", out.status).into());
        }
        cli.version = str::from_utf8(out.stdout.as_slice())
            .unwrap()
            .trim()
            .to_owned();
        log::info!("using {}", cli.version);
        Ok(cli)
    }

    fn command(&self) -> Command {
        Command::new("docker")
    }
}
