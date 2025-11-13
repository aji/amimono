use std::{collections::HashMap, net::SocketAddr};

use log::{debug, error, warn};

use crate::{
    AppConfig, JobConfig, Label,
    toml::{BindingToml, BindingTypeToml, BindingsToml},
};

pub trait BindingAllocator {
    fn next_http(&mut self, job: &JobConfig) -> (SocketAddr, String);
}

#[derive(Copy, Clone, Debug)]
pub enum BindingType {
    None,
    Http,
}

impl BindingType {
    pub fn to_toml(&self) -> BindingTypeToml {
        match self {
            BindingType::None => BindingTypeToml::None,
            BindingType::Http => BindingTypeToml::Http,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Binding {
    None,
    Http(SocketAddr, String),
}

impl Binding {
    fn compatible(&self, ty: BindingType) -> bool {
        match (self, ty) {
            (Binding::None, BindingType::None) | (Binding::Http(_, _), BindingType::Http) => true,
            _ => false,
        }
    }
}

impl Into<Binding> for &BindingToml {
    fn into(self) -> Binding {
        match self {
            BindingToml::None => todo!(),
            BindingToml::Http { internal, external } => {
                Binding::Http(internal.parse().unwrap(), external.clone())
            }
        }
    }
}

pub struct Bindings {
    pub(crate) comps: HashMap<Label, Binding>,
}

impl Bindings {
    pub fn new<A: BindingAllocator>(cf: &AppConfig, mut alloc: A) -> Bindings {
        let mut comps = HashMap::new();
        for job in cf.jobs() {
            for comp in job.components() {
                let bind = match comp.binding() {
                    BindingType::None => Binding::None,
                    BindingType::Http => {
                        let (addr, endpoint) = alloc.next_http(job);
                        Binding::Http(addr, endpoint)
                    }
                };
                debug!("binding: {} -> {:?}", comp.label(), bind);
                comps.insert(comp.label(), bind);
            }
        }
        Bindings { comps }
    }

    pub fn from_file<P: AsRef<std::path::Path>>(cf: &AppConfig, path: P) -> Result<Bindings, ()> {
        let path = AsRef::as_ref(&path);
        debug!("loading bindings from {:?}", path);
        let data = match std::fs::read(path) {
            Ok(x) => x,
            Err(_) => {
                error!("could not read bindings from {:?}", path);
                return Err(());
            }
        };
        let bindings = match toml::from_slice(data.as_slice()) {
            Ok(x) => x,
            Err(e) => {
                error!("failed to parse bindings from {:?}: {}", path, e);
                return Err(());
            }
        };
        Ok(Bindings::from_toml(cf, &bindings)?)
    }

    pub fn from_toml(cf: &AppConfig, toml: &BindingsToml) -> Result<Bindings, ()> {
        let mut comps = HashMap::new();
        let mut errors = false;
        for comp in cf.components() {
            let binding = match toml.components.get(comp.label()) {
                Some(x) => x.into(),
                None => {
                    warn!(
                        "config missing binding for {}, defaulting to None",
                        comp.label()
                    );
                    Binding::None
                }
            };
            if binding.compatible(comp.binding()) {
                debug!("binding: {} -> {:?}", comp.label(), binding);
                comps.insert(comp.label(), binding);
            } else {
                error!(
                    "binding {:?} for {} incompatible with component type {:?}",
                    binding,
                    comp.label(),
                    comp.binding()
                );
                errors = true;
            }
        }
        if errors {
            Err(())
        } else {
            Ok(Bindings { comps })
        }
    }
}
