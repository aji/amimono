use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    thread,
};

use crate::{
    AppConfig, JobConfig,
    binding::{BindingAllocator, Bindings},
    job::run_job,
};

pub fn run_local(cf: AppConfig) {
    let cf = Arc::new(cf);
    let bindings = Arc::new(Bindings::new(&cf, LocalBindingAllocator::new()));

    let mut threads = Vec::new();
    for job in cf.jobs() {
        threads.push(thread::spawn({
            let cf = cf.clone();
            let bindings = bindings.clone();
            let label = job.label();
            move || run_job(&cf, &bindings, label)
        }));
    }
    for th in threads.into_iter() {
        th.join().unwrap();
    }
}

struct LocalBindingAllocator {
    next_port: u16,
}

impl LocalBindingAllocator {
    fn new() -> LocalBindingAllocator {
        LocalBindingAllocator { next_port: 9000 }
    }
}

impl BindingAllocator for LocalBindingAllocator {
    fn next_http(&mut self, _job: &JobConfig) -> (SocketAddr, String) {
        let port = self.next_port;
        self.next_port += 1;
        let addr = (Ipv4Addr::LOCALHOST, port).into();
        let url = format!("http://localhost:{}", port);
        (addr, url)
    }
}
