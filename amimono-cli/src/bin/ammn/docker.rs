use std::{
    error::Error,
    process::{Command, Stdio},
};

use flate2::Compression;

use crate::project::Project;

pub fn go(proj: &dyn Project) {
    let cli = match DockerCli::new() {
        Ok(x) => x,
        Err(e) => crate::fatal!("could not create Docker CLI: {}", e),
    };

    let name = proj.name();
    let path = proj.build_local();
    log::info!("{:?} at {:?}", name, path.to_str());

    let mut build = {
        let mut cmd = cli.command();
        cmd.arg("build")
            .arg("-t")
            .arg(format!("{}/latest", name))
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        match cmd.spawn() {
            Ok(x) => x,
            Err(e) => crate::fatal!("failed to invoke docker build: {}", e),
        }
    };

    let _ = {
        let child_stdin = build.stdin.take().expect("no stdin handle on child");
        let gz_writer = flate2::write::GzEncoder::new(child_stdin, Compression::fast());
        let gz_writer = {
            let mut tar_writer = tar::Builder::new(gz_writer);
            let dockerfile = gen_dockerfile(&name);
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o644);
            header.set_size(dockerfile.len() as u64);
            header.set_cksum();
            tar_writer
                .append_data(&mut header, "Dockerfile", &dockerfile[..])
                .unwrap();
            tar_writer.append_path_with_name(path, name).unwrap();
            tar_writer.into_inner().unwrap()
        };
        gz_writer.finish().unwrap();
    };

    let status = match build.wait() {
        Ok(x) => x,
        Err(e) => crate::fatal!("docker build failed: {}", e),
    };
    if !status.success() {
        crate::fatal!("docker build failed with {}", status);
    }
}

fn gen_dockerfile<S: AsRef<str>>(name: S) -> Vec<u8> {
    let name = name.as_ref();
    let s = format!(
        "FROM arm64v8/busybox:glibc\n\
        COPY ./{} /{}\n\
        CMD [\"/{}\"]\n",
        name, name, name
    );
    s.into_bytes()
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
            return Err(format!("docker --version failed with {}", out.status).into());
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
