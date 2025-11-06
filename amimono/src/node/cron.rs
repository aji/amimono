use std::thread;

use log::info;

use super::NodeContext;
use crate::Cron;

pub fn cron_main<C: Cron>(ctx: NodeContext) {
    info!("in cron_main for {}", C::LABEL);
    let job = C::init();
    loop {
        job.fire(&ctx);
        thread::sleep(C::INTERVAL);
    }
}
