use crate::{
    config::{Config, TargetConfig},
    project::Project,
};

pub trait Target {
    fn deploy(&self, proj: &dyn Project);
}

pub struct KubernetesTarget(pub String);
impl Target for KubernetesTarget {
    fn deploy(&self, _proj: &dyn Project) {
        todo!()
    }
}

pub fn get<S: AsRef<str>>(cf: &Config, tgt_id: S) -> Box<dyn Target> {
    match cf.target.get(tgt_id.as_ref()) {
        Some(TargetConfig::Kubernetes { cluster }) => Box::new(KubernetesTarget(cluster.clone())),
        None => crate::fatal!("no such target: {}", tgt_id.as_ref()),
    }
}
