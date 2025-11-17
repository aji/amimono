use std::thread;

use futures::future::join_all;

use crate::runtime;

#[tokio::main]
pub async fn run_job(job: &str) {
    log::info!("starting job {}", job);

    let mut comps = Vec::new();
    for comp in runtime::config().job(job).components() {
        comps.push(runtime::CURRENT_LABEL.scope(comp.label(), async {
            log::info!("starting component {}", comp.label());
            comp.start().await
        }));
    }

    join_all(comps).await;
}

pub fn run_all() {
    let mut threads = Vec::new();
    for job in runtime::config().jobs() {
        threads.push(thread::spawn({
            let label = job.label();
            move || run_job(label)
        }));
    }
    for th in threads.into_iter() {
        th.join().unwrap();
    }
}
