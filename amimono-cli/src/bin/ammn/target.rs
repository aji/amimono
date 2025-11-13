use crate::project::Project;

pub trait Target {
    fn deploy(&self, proj: &dyn Project);
}

pub struct KubernetesTarget(pub String);
impl Target for KubernetesTarget {
    fn deploy(&self, _proj: &dyn Project) {
        todo!()
    }
}
