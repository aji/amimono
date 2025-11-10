use futures::future::join_all;

use crate::{AppConfig, Label, Runtime};

pub fn run_job(label: Label, cf: &AppConfig) {
    let tokio_rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    tokio_rt.block_on(async {
        let rt: Runtime = Runtime::new(label, cf);
        join_all(cf.job(label).components().map(|comp| {
            let rt: Runtime = rt.for_component(comp.label());
            comp.main_async(rt)
        }))
        .await
    });
}
