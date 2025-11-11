use std::{thread, time::Duration};

use amimono::{Component, Runtime, async_component_fn};
use amimono_rpc::{Rpc, RpcClient};
use log::info;
use rand::Rng;

use crate::doubler::Doubler;

async fn driver_main(rt: Runtime) {
    let doubler: RpcClient<Doubler> = Doubler::client(&rt);
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let a = rand::rng().random_range(10..50);
        info!("doubling {}", a);
        let b = doubler.call(a).await;
        info!("got {}", b);
    }
}

pub fn component() -> Component {
    async_component_fn("driver", driver_main)
}
