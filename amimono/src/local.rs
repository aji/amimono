use std::{
    collections::HashMap,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
    thread,
};

use crate::{BindingType, Component, Configuration, Context, LocalBinding, RemoteBinding};

pub struct LocalConfig {
    bindings: HashMap<String, RemoteBinding>,
}

pub struct LocalConfigBuilder {
    next_port: u16,
    cf: LocalConfig,
}

impl LocalConfigBuilder {
    pub fn new() -> LocalConfigBuilder {
        LocalConfigBuilder {
            next_port: 9000,
            cf: LocalConfig {
                bindings: HashMap::new(),
            },
        }
    }

    pub fn build(self) -> LocalConfig {
        self.cf
    }
}

impl Configuration for LocalConfigBuilder {
    fn place<C: Component>(&mut self) {
        let binding = match C::BINDING {
            BindingType::None => RemoteBinding::None,
            BindingType::TCP(n) => {
                let binds: Vec<SocketAddr> = (0..n as u16)
                    .map(|i| ([127, 0, 0, 1], self.next_port + i).into())
                    .collect();
                self.next_port += n as u16;
                RemoteBinding::TCP(binds)
            }
        };
        let is_none = self
            .cf
            .bindings
            .insert(C::LABEL.to_owned(), binding)
            .is_none();
        assert!(is_none);
    }
}

pub struct LocalLauncher {
    cf: Arc<LocalConfig>,
    threads: Vec<thread::JoinHandle<()>>,
}

impl LocalLauncher {
    pub fn new(cf: LocalConfig) -> LocalLauncher {
        LocalLauncher {
            cf: Arc::new(cf),
            threads: Vec::new(),
        }
    }

    pub fn finish(self) {
        for thread in self.threads {
            thread.join().expect("a thread panicked");
        }
    }
}

impl Configuration for LocalLauncher {
    fn place<C: Component>(&mut self) {
        let cf = self.cf.clone();
        let join = thread::spawn(move || {
            let ctx = LocalContext::new::<C>(cf);
            C::main(ctx);
        });
        self.threads.push(join);
    }
}

pub struct LocalContext {
    binding: LocalBinding,
    cf: Arc<LocalConfig>,
}

fn to_localhost(addr: &SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V4(v4) => {
            let mut v4 = v4.clone();
            v4.set_ip(Ipv4Addr::LOCALHOST);
            SocketAddr::V4(v4)
        }
        SocketAddr::V6(v6) => {
            let mut v6 = v6.clone();
            v6.set_ip(Ipv6Addr::LOCALHOST);
            SocketAddr::V6(v6)
        }
    }
}

impl LocalContext {
    pub fn new<C: Component>(cf: Arc<LocalConfig>) -> LocalContext {
        let binding = match cf.bindings.get(C::LABEL).unwrap() {
            RemoteBinding::None => LocalBinding::None,
            RemoteBinding::TCP(items) => {
                LocalBinding::TCP(items.iter().map(to_localhost).collect())
            }
        };
        LocalContext { binding, cf }
    }
}

impl Context for LocalContext {
    fn binding(&self) -> &LocalBinding {
        &self.binding
    }
    fn locate<D: Component>(&self) -> &RemoteBinding {
        self.cf.bindings.get(D::LABEL).unwrap()
    }
}
