use log::info;

use super::NodeContext;
use crate::RPC;

pub fn rpc_main<C: RPC>(ctx: NodeContext) {
    info!("in rpc_main for {}", C::LABEL);
}
