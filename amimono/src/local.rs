use std::{collections::HashMap, sync::Arc, thread};

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
                let binds: Vec<(String, u16)> = (0..n as u16)
                    .map(|i| ("localhost".to_owned(), self.next_port + i))
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
            C::main(&ctx);
        });
        self.threads.push(join);
    }
}

pub struct LocalContext {
    binding: LocalBinding,
    cf: Arc<LocalConfig>,
}

impl LocalContext {
    pub fn new<C: Component>(cf: Arc<LocalConfig>) -> LocalContext {
        let binding = match cf.bindings.get(C::LABEL).unwrap() {
            RemoteBinding::None => LocalBinding::None,
            RemoteBinding::TCP(items) => {
                LocalBinding::TCP(items.iter().map(|(_, port)| *port).collect())
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
