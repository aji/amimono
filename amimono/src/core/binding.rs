use std::{collections::HashMap, net::SocketAddr};

use crate::{AppConfig, JobConfig, Label};

pub trait BindingAllocator {
    fn next_http(&mut self, job: &JobConfig) -> (SocketAddr, String);
}

#[derive(Copy, Clone)]
pub enum BindingType {
    None,
    HTTP,
}

#[derive(Clone)]
pub enum Binding {
    None,
    HTTP(SocketAddr, String),
}

pub struct Bindings {
    comps: HashMap<Label, Binding>,
}

impl Bindings {
    pub fn new<A: BindingAllocator>(cf: &AppConfig, mut alloc: A) -> Bindings {
        let mut comps = HashMap::new();
        for job in cf.jobs() {
            for comp in job.components() {
                let bind = match comp.binding() {
                    BindingType::None => Binding::None,
                    BindingType::HTTP => {
                        let (addr, endpoint) = alloc.next_http(job);
                        Binding::HTTP(addr, endpoint)
                    }
                };
                comps.insert(comp.label(), bind);
            }
        }
        Bindings { comps }
    }

    pub fn get(&self, comp: Label) -> &Binding {
        self.comps.get(comp).unwrap()
    }
}
