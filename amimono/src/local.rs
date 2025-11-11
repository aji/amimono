use std::{sync::Arc, thread};

use crate::{AppConfig, job::run_job};

pub fn run_local(cf: AppConfig) {
    let cf = Arc::new(cf);

    let mut threads = Vec::new();
    for job in cf.jobs() {
        threads.push(thread::spawn({
            let cf = cf.clone();
            let label = job.label();
            move || run_job(&cf, label)
        }));
    }
    for th in threads.into_iter() {
        th.join().unwrap();
    }
}
