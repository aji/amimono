use std::sync::Arc;

use futures::future::join_all;

use crate::{AppConfig, Label, Runtime, binding::Bindings};

#[tokio::main(flavor = "current_thread")]
pub async fn run_job(cf: &AppConfig, bindings: Arc<Bindings>, job: Label) {
    let rt = Runtime::new(cf, bindings, job);

    let mut comps = Vec::new();
    for comp in cf.job(job).components() {
        let rt = rt.place(comp.label());
        comps.push(comp.start(rt));
    }

    join_all(comps).await;
}
