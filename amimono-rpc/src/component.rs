use std::sync::Arc;

use amimono::{Component, Label, Runtime, async_component_fn};

use crate::{server::run_server, traits::Rpc};

pub fn rpc_component<C: Rpc>(label: Label) -> Component {
    async_component_fn(label, async |rt| {
        let rt: Arc<Runtime> = Arc::new(rt);
        let job: Arc<C> = Arc::new(C::start(&rt).await);
        let rt_local = rt.clone();
        let job_local = job.clone();
        rt.bind_local(async move |q: C::Request| job_local.handle(&*rt_local, q).await)
            .await;
        run_server(rt, job).await;
    })
}
