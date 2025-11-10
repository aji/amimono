use std::sync::Arc;

use crate::{AppConfig, Label};

#[derive(Clone)]
enum RuntimeScope {
    Job(Label),
    Component(Label),
}

#[derive(Clone)]
pub struct Runtime {
    scope: RuntimeScope,
    data: Arc<RuntimeData>,
}

struct RuntimeData {}

impl Runtime {
    pub fn new(job: Label, cf: &AppConfig) -> Runtime {
        for comp in cf.job(job).components() {}
        Runtime {
            scope: RuntimeScope::Job(job),
            data: Arc::new(RuntimeData {}),
        }
    }

    pub fn for_component(&self, comp: Label) -> Runtime {
        let next = match self.scope {
            RuntimeScope::Job(_) => RuntimeScope::Component(comp),
            RuntimeScope::Component(_) => panic!(),
        };
        Runtime {
            scope: next,
            data: self.data.clone(),
        }
    }
}
