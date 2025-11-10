use std::{sync::Arc, thread};

use crate::{AppConfig, binding::BindingAllocator, job::run_job};

struct LocalBindingAllocator;

impl BindingAllocator for LocalBindingAllocator {
    fn next_http(&mut self) -> (std::net::SocketAddr, String) {
        todo!()
    }
}

pub fn run_local(cf: AppConfig) {
    let cf = Arc::new(cf);
    let mut joins: Vec<thread::JoinHandle<()>> = Vec::new();
    for job in cf.jobs() {
        let label = job.label();
        let cf = cf.clone();
        joins.push(thread::spawn(move || {
            run_job(label, &*cf);
        }));
    }
    for join in joins {
        join.join().unwrap();
    }
}
