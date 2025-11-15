use futures::future::join_all;

use crate::{AppConfig, Runtime, binding::Bindings};

#[tokio::main]
pub async fn run_job(cf: &AppConfig, bindings: &Bindings, job: &str) {
    let rt = Runtime::new(cf, bindings, job);
    log::info!("starting job {}", job);

    let mut comps = Vec::new();
    for comp in cf.job(job).components() {
        let rt = rt.place(comp.label());
        comps.push(async {
            log::info!("starting component {}", comp.label());
            comp.start(rt).await
        });
    }

    join_all(comps).await;
}
