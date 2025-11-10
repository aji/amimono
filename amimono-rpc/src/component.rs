use amimono::{Component, Label, async_component_fn};

use crate::{server::run_server, traits::Rpc};

pub fn rpc_component<C: Rpc>(label: Label) -> Component {
    async_component_fn(label, async |rt| {
        run_server(&rt, C::start(&rt).await).await;
    })
}
