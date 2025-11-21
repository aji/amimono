use std::{
    hash::{Hash, Hasher},
    io::{self, Write},
};

use crate::{
    config::{DumpBinding, DumpConfig, TargetConfig},
    project::Project,
};

pub enum Target {
    Kubernetes { cluster: String, image: String },
}

impl Target {
    pub fn from_config(cf: &crate::config::Config, target: &str, image: Option<&str>) -> Self {
        match cf.target.get(target) {
            Some(TargetConfig::Kubernetes { cluster }) => Target::Kubernetes {
                cluster: cluster.clone(),
                image: image.map(|s| s.to_owned()).unwrap_or_else(|| {
                    crate::fatal!("image must be specified for Kubernetes target {}", target)
                }),
            },
            None => todo!(),
        }
    }

    pub fn deploy(&self, _proj: &Project) {
        match self {
            Target::Kubernetes { cluster, image } => deploy_kubernetes(cluster, image),
        }
    }
}

struct KubernetesWriter<'w, W> {
    out: &'w mut W,
    image: &'w str,
    rev: String,
}

impl<'w, W: io::Write> KubernetesWriter<'w, W> {
    fn new(out: &'w mut W, image: &'w str) -> Self {
        let rev = {
            let mut hasher = fnv::FnvHasher::default();
            image.hash(&mut hasher);
            format!("{:08x}", hasher.finish() & 0xffffffff)
        };
        KubernetesWriter { out, image, rev }
    }

    fn add_dump_config_job(&mut self) -> io::Result<()> {
        let image = self.image;
        writeln!(self.out, "---")?;
        writeln!(self.out, "apiVersion: batch/v1")?;
        writeln!(self.out, "kind: Job")?;
        writeln!(self.out, "metadata:")?;
        writeln!(self.out, "  name: dump-config")?;
        writeln!(self.out, "  labels:")?;
        writeln!(self.out, "    amimono-rev: {}", self.rev)?;
        writeln!(self.out, "spec:")?;
        writeln!(self.out, "  template:")?;
        writeln!(self.out, "    spec:")?;
        writeln!(self.out, "      containers:")?;
        writeln!(self.out, "        - name: dump-config")?;
        writeln!(self.out, "          image: {}", image)?;
        writeln!(self.out, "          args: [\"--dump-config\"]")?;
        writeln!(self.out, "          env:")?;
        writeln!(self.out, "            - name: RUST_LOG")?;
        writeln!(self.out, "              value: warn")?;
        writeln!(self.out, "      restartPolicy: Never")?;
        Ok(())
    }

    fn add_deployment(&mut self, job: &str, ports: &[u16]) -> io::Result<()> {
        writeln!(self.out, "---")?;
        writeln!(self.out, "apiVersion: apps/v1")?;
        writeln!(self.out, "kind: Deployment")?;
        writeln!(self.out, "metadata:")?;
        writeln!(self.out, "  name: {}", job)?;
        writeln!(self.out, "  labels:")?;
        writeln!(self.out, "    amimono-job: {}", job)?;
        writeln!(self.out, "    amimono-rev: {}", self.rev)?;
        writeln!(self.out, "spec:")?;
        writeln!(self.out, "  replicas: 1")?;
        writeln!(self.out, "  selector:")?;
        writeln!(self.out, "    matchLabels:")?;
        writeln!(self.out, "      amimono-job: {}", job)?;
        writeln!(self.out, "  template:")?;
        writeln!(self.out, "    metadata:")?;
        writeln!(self.out, "      labels:")?;
        writeln!(self.out, "        amimono-job: {}", job)?;
        writeln!(self.out, "        amimono-rev: {}", self.rev)?;
        writeln!(self.out, "    spec:")?;
        writeln!(self.out, "      containers:")?;
        writeln!(self.out, "        - name: {}", job)?;
        writeln!(self.out, "          image: {}", self.image)?;
        if !ports.is_empty() {
            writeln!(self.out, "          ports:")?;
            for port in ports {
                writeln!(self.out, "            - containerPort: {}", port)?;
            }
        }
        writeln!(self.out, "          args: [\"--job\", \"{}\"]", job)?;
        writeln!(self.out, "          env:")?;
        writeln!(self.out, "            - name: RUST_LOG")?;
        writeln!(self.out, "              value: info")?;
        Ok(())
    }

    fn add_service(&mut self, job: &str, component: &str, port: u16) -> io::Result<()> {
        writeln!(self.out, "---")?;
        writeln!(self.out, "apiVersion: v1")?;
        writeln!(self.out, "kind: Service")?;
        writeln!(self.out, "metadata:")?;
        writeln!(self.out, "  name: {}", component)?;
        writeln!(self.out, "  labels:")?;
        writeln!(self.out, "    amimono-job: {}", job)?;
        writeln!(self.out, "    amimono-component: {}", component)?;
        writeln!(self.out, "    amimono-rev: {}", self.rev)?;
        writeln!(self.out, "spec:")?;
        writeln!(self.out, "  selector:")?;
        writeln!(self.out, "    amimono-job: {}", job)?;
        writeln!(self.out, "    amimono-rev: {}", self.rev)?;
        writeln!(self.out, "  type: NodePort")?;
        writeln!(self.out, "  ports:")?;
        writeln!(self.out, "    - protocol: TCP")?;
        writeln!(self.out, "      port: {}", port)?;
        writeln!(self.out, "      targetPort: {}", port)?;
        Ok(())
    }
}

struct KubernetesClient {
    cluster: String,
    image: String,
}

impl KubernetesClient {
    fn new(cluster: &str, image: &str) -> Self {
        KubernetesClient {
            cluster: cluster.to_owned(),
            image: image.to_owned(),
        }
    }

    fn get_yaml<F>(&self, cb: F) -> io::Result<String>
    where
        F: FnOnce(&mut KubernetesWriter<Vec<u8>>) -> io::Result<()>,
    {
        let mut out: Vec<u8> = Vec::new();
        let mut writer = KubernetesWriter::new(&mut out, &self.image);
        cb(&mut writer)?;
        Ok(String::from_utf8(out).unwrap())
    }

    fn do_delete(&self, yaml: &str) -> io::Result<()> {
        let mut cmd = std::process::Command::new("kubectl");
        cmd.arg("--context").arg(&self.cluster);
        cmd.arg("delete")
            .arg("-f")
            .arg("-")
            .arg("--wait=true")
            .arg("--ignore-not-found=true");
        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;
        {
            let stdin = child.stdin.as_mut().unwrap();
            stdin.write_all(yaml.as_bytes())?;
        }
        let status = child.wait()?;
        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("kubectl exited with status {}", status),
            ));
        }
        Ok(())
    }

    fn do_apply(&self, yaml: &str) -> io::Result<()> {
        let mut cmd = std::process::Command::new("kubectl");
        cmd.arg("--context").arg(&self.cluster);
        cmd.arg("apply").arg("-f").arg("-");
        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;
        {
            let stdin = child.stdin.as_mut().unwrap();
            stdin.write_all(yaml.as_bytes())?;
        }
        let status = child.wait()?;
        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("kubectl exited with status {}", status),
            ));
        }
        Ok(())
    }

    fn do_wait_for_job(&self, job: &str) -> io::Result<()> {
        let mut cmd = std::process::Command::new("kubectl");
        cmd.arg("--context").arg(&self.cluster);
        cmd.arg("wait")
            .arg("--for=condition=complete")
            .arg("--timeout=60s")
            .arg("job/".to_string() + job);
        let output = cmd.output()?;
        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("kubectl exited with status {}", output.status),
            ));
        }
        Ok(())
    }

    fn do_get_job_output(&self, job: &str) -> io::Result<Vec<u8>> {
        let mut cmd = std::process::Command::new("kubectl");
        cmd.arg("--context").arg(&self.cluster);
        cmd.arg("logs").arg("job/".to_string() + job);
        let output = cmd.output()?;
        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("kubectl exited with status {}", output.status),
            ));
        }
        Ok(output.stdout)
    }

    fn get_config(&self) -> io::Result<DumpConfig> {
        let yaml = self.get_yaml(|w| w.add_dump_config_job())?;

        log::info!("cleaning up any existing dump-config jobs...");
        self.do_delete(&yaml)?;

        log::info!("creating dump-config job...");
        self.do_apply(&yaml)?;

        log::info!("waiting for dump-config job to complete...");
        self.do_wait_for_job("dump-config")?;

        log::info!("getting dump-config output");
        let output = self.do_get_job_output("dump-config")?;

        serde_json::from_slice(&output[..]).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("failed to parse dump config JSON: {}", e),
            )
        })
    }
}

fn deploy_kubernetes(cluster: &str, image: &str) {
    let client = KubernetesClient::new(cluster, image);

    let cf = match client.get_config() {
        Ok(c) => c,
        Err(e) => crate::fatal!("failed to get app config from cluster {}: {}", cluster, e),
    };

    log::info!("generating Kubernetes objects from app config...");
    let yaml = client.get_yaml(|w| {
        for (job_label, job) in cf.jobs.iter() {
            let ports = job
                .components
                .values()
                .flat_map(|x| match x.binding {
                    DumpBinding::Http { port } => Some(port),
                    _ => None,
                })
                .filter(|&p| p != 0)
                .collect::<Vec<u16>>();
            w.add_deployment(job_label.as_str(), ports.as_slice())?;
        }
        for (job_label, job) in cf.jobs.iter() {
            for (comp_label, comp) in job.components.iter() {
                let port = match comp.binding {
                    DumpBinding::Http { port } => Some(port),
                    _ => None,
                };
                if let Some(port) = port {
                    w.add_service(job_label.as_str(), comp_label.as_str(), port)?;
                }
            }
        }
        Ok(())
    });
    let yaml = match yaml {
        Ok(y) => y,
        Err(e) => crate::fatal!(
            "failed to generate Kubernetes objects for cluster {}: {}",
            cluster,
            e
        ),
    };

    log::info!("running kubectl apply...");
    if let Err(e) = client.do_apply(&yaml) {
        crate::fatal!("apply failed: {}", e);
    }

    log::info!("all done!");
}
