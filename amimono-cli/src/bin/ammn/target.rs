use std::{
    collections::HashMap,
    io::{self, Write},
};

use amimono_schemas::{DumpBinding, DumpConfig};

use crate::{config::TargetConfig, project::Project};

#[allow(private_interfaces)]
pub enum Target {
    Kubernetes(KubernetesTarget),
}

impl Target {
    pub fn from_config(cf: &crate::config::Config, target: &str) -> Self {
        match cf.target.get(target) {
            Some(TargetConfig::Kubernetes {
                context,
                image,
                env,
            }) => {
                let tgt = KubernetesTarget {
                    context: context.clone(),
                    env: env.to_owned().unwrap_or_default(),
                    image: image.to_owned(),
                };
                Target::Kubernetes(tgt)
            }
            None => {
                crate::fatal!(
                    "unknown target. available targets: {}",
                    cf.target
                        .keys()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
    }

    pub fn deploy(&self, _proj: &Project) {
        match self {
            Target::Kubernetes(target) => target.deploy(),
        }
    }
}

struct KubernetesTarget {
    context: String,
    env: HashMap<String, String>,
    image: String,
}

impl KubernetesTarget {
    fn get_yaml<F>(&self, cb: F) -> io::Result<String>
    where
        F: FnOnce(&mut KubernetesWriter<Vec<u8>>) -> io::Result<()>,
    {
        let mut out: Vec<u8> = Vec::new();
        let mut writer = KubernetesWriter::new(&self, &mut out);
        cb(&mut writer)?;
        Ok(String::from_utf8(out).unwrap())
    }

    fn do_delete(&self, yaml: &str) -> io::Result<()> {
        let mut cmd = std::process::Command::new("kubectl");
        cmd.arg("--context").arg(&self.context);
        cmd.arg("delete")
            .arg("-f")
            .arg("-")
            .arg("--wait=true")
            .arg("--ignore-not-found=true");
        log::debug!("kubectl delete: {}", yaml.trim_end());
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
        cmd.arg("--context").arg(&self.context);
        cmd.arg("apply").arg("-f").arg("-");
        log::debug!("kubectl apply: {}", yaml.trim_end());
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
        cmd.arg("--context").arg(&self.context);
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
        cmd.arg("--context").arg(&self.context);
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

    fn get_app_config(&self) -> io::Result<DumpConfig> {
        let yaml = self.get_yaml(|w| w.add_dump_config_job())?;

        log::info!("cleaning up any existing dump-config jobs...");
        self.do_delete(&yaml)?;

        log::info!("creating dump-config job...");
        self.do_apply(&yaml)?;

        log::info!("waiting for dump-config job to complete...");
        self.do_wait_for_job("dump-config")?;

        log::info!("getting dump-config output");
        let output = self.do_get_job_output("dump-config")?;

        log::info!("cleaning up dump-config job...");
        self.do_delete(&yaml)?;

        serde_json::from_slice(&output[..]).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("failed to parse dump config JSON: {}", e),
            )
        })
    }

    fn deploy(&self) {
        let cf = match self.get_app_config() {
            Ok(c) => c,
            Err(e) => crate::fatal!(
                "failed to get app config from cluster {}: {}",
                self.context,
                e
            ),
        };

        log::info!("generating Kubernetes objects from app config...");
        let yaml = self.get_yaml(|w| {
            for (job_label, job) in cf.jobs.iter() {
                for (comp_label, comp) in job.components.iter() {
                    let port = match comp.binding {
                        DumpBinding::Rpc => Some(9099),
                        DumpBinding::Tcp { port } => Some(port),
                        _ => None,
                    };
                    if let Some(port) = port {
                        w.add_service(&job_label, &cf.revision, &comp_label, port)?;
                    }
                }
            }
            for (job_label, job) in cf.jobs.iter() {
                let ports = job
                    .components
                    .values()
                    .flat_map(|x| match x.binding {
                        DumpBinding::Rpc => Some(9099),
                        DumpBinding::Tcp { port } => Some(port),
                        _ => None,
                    })
                    .filter(|&p| p != 0)
                    .collect::<Vec<u16>>();
                if job.is_stateful {
                    w.add_statefulset(&job_label, &cf.revision, &ports[..])?;
                } else {
                    w.add_deployment(&job_label, &cf.revision, &ports[..])?;
                }
            }
            Ok(())
        });
        let yaml = match yaml {
            Ok(y) => y,
            Err(e) => crate::fatal!(
                "failed to generate Kubernetes objects for context {}: {}",
                self.context,
                e
            ),
        };

        log::info!("running kubectl apply...");
        if let Err(e) = self.do_apply(&yaml) {
            crate::fatal!("apply failed: {}", e);
        }

        log::info!("all done!");
    }
}

struct KubernetesWriter<'w, W> {
    tgt: &'w KubernetesTarget,
    out: &'w mut W,
}

impl<'w, W: io::Write> KubernetesWriter<'w, W> {
    fn new(tgt: &'w KubernetesTarget, out: &'w mut W) -> Self {
        KubernetesWriter { tgt, out }
    }

    fn add_dump_config_job(&mut self) -> io::Result<()> {
        writeln!(self.out, "---")?;
        writeln!(self.out, "apiVersion: batch/v1")?;
        writeln!(self.out, "kind: Job")?;
        writeln!(self.out, "metadata:")?;
        writeln!(self.out, "  name: dump-config")?;
        writeln!(self.out, "spec:")?;
        writeln!(self.out, "  template:")?;
        writeln!(self.out, "    spec:")?;
        writeln!(self.out, "      containers:")?;
        writeln!(self.out, "        - name: dump-config")?;
        writeln!(self.out, "          image: {}", self.tgt.image)?;
        writeln!(self.out, "          imagePullPolicy: IfNotPresent")?;
        writeln!(self.out, "          args: [\"--dump-config\"]")?;
        writeln!(self.out, "          env:")?;
        writeln!(self.out, "            - name: RUST_LOG")?;
        writeln!(self.out, "              value: warn")?;
        writeln!(self.out, "            - name: RUST_BACKTRACE")?;
        writeln!(self.out, "              value: \"1\"")?;
        writeln!(self.out, "      restartPolicy: Never")?;
        Ok(())
    }

    fn add_podtemplatespec(&mut self, job: &str, ports: &[u16]) -> io::Result<()> {
        writeln!(self.out, "      containers:")?;
        writeln!(self.out, "        - name: {}", job)?;
        writeln!(self.out, "          image: {}", self.tgt.image)?;
        writeln!(self.out, "          imagePullPolicy: IfNotPresent")?;
        if !ports.is_empty() {
            writeln!(self.out, "          ports:")?;
            for port in ports {
                writeln!(self.out, "            - containerPort: {}", port)?;
            }
        }
        writeln!(self.out, "          args: [\"--job\", \"{}\"]", job)?;
        if !self.tgt.env.is_empty() {
            writeln!(self.out, "          env:")?;
            for (key, value) in self.tgt.env.iter() {
                assert!(!value.contains('"'));
                writeln!(self.out, "            - name: {}", key)?;
                writeln!(self.out, "              value: \"{}\"", value)?;
            }
        }
        Ok(())
    }

    fn add_deployment(&mut self, job: &str, rev: &str, ports: &[u16]) -> io::Result<()> {
        writeln!(self.out, "---")?;
        writeln!(self.out, "apiVersion: apps/v1")?;
        writeln!(self.out, "kind: Deployment")?;
        writeln!(self.out, "metadata:")?;
        writeln!(self.out, "  name: {}", job)?;
        writeln!(self.out, "  labels:")?;
        writeln!(self.out, "    amimono-job: {}", job)?;
        writeln!(self.out, "    amimono-rev: \"{}\"", rev)?;
        writeln!(self.out, "spec:")?;
        writeln!(self.out, "  replicas: 1")?;
        writeln!(self.out, "  selector:")?;
        writeln!(self.out, "    matchLabels:")?;
        writeln!(self.out, "      amimono-job: {}", job)?;
        writeln!(self.out, "  template:")?;
        writeln!(self.out, "    metadata:")?;
        writeln!(self.out, "      labels:")?;
        writeln!(self.out, "        amimono-job: {}", job)?;
        writeln!(self.out, "        amimono-rev: \"{}\"", rev)?;
        writeln!(self.out, "    spec:")?;
        self.add_podtemplatespec(job, ports)?;
        Ok(())
    }

    fn add_statefulset(&mut self, job: &str, rev: &str, ports: &[u16]) -> io::Result<()> {
        writeln!(self.out, "---")?;
        writeln!(self.out, "apiVersion: apps/v1")?;
        writeln!(self.out, "kind: StatefulSet")?;
        writeln!(self.out, "metadata:")?;
        writeln!(self.out, "  name: {}", job)?;
        writeln!(self.out, "  labels:")?;
        writeln!(self.out, "    amimono-job: {}", job)?;
        writeln!(self.out, "    amimono-rev: \"{}\"", rev)?;
        writeln!(self.out, "spec:")?;
        writeln!(self.out, "  serviceName: {}", job)?;
        writeln!(self.out, "  replicas: 1")?;
        writeln!(self.out, "  selector:")?;
        writeln!(self.out, "    matchLabels:")?;
        writeln!(self.out, "      amimono-job: {}", job)?;
        writeln!(self.out, "  template:")?;
        writeln!(self.out, "    metadata:")?;
        writeln!(self.out, "      labels:")?;
        writeln!(self.out, "        amimono-job: {}", job)?;
        writeln!(self.out, "        amimono-rev: \"{}\"", rev)?;
        writeln!(self.out, "    spec:")?;
        self.add_podtemplatespec(job, ports)?;
        Ok(())
    }

    fn add_service(&mut self, job: &str, _rev: &str, component: &str, port: u16) -> io::Result<()> {
        writeln!(self.out, "---")?;
        writeln!(self.out, "apiVersion: v1")?;
        writeln!(self.out, "kind: Service")?;
        writeln!(self.out, "metadata:")?;
        writeln!(self.out, "  name: {}", component)?;
        writeln!(self.out, "  labels:")?;
        writeln!(self.out, "    amimono-component: {}", component)?;
        writeln!(self.out, "spec:")?;
        writeln!(self.out, "  selector:")?;
        writeln!(self.out, "    amimono-job: {}", job)?;
        writeln!(self.out, "  type: NodePort")?;
        writeln!(self.out, "  ports:")?;
        writeln!(self.out, "    - protocol: TCP")?;
        writeln!(self.out, "      port: {}", port)?;
        writeln!(self.out, "      targetPort: {}", port)?;
        Ok(())
    }
}
