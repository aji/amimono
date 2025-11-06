use super::NodeConfig;
use crate::Context;

pub struct NodeContext {
    cf: NodeConfig,
}

impl NodeContext {
    pub fn new(cf: NodeConfig) -> NodeContext {
        NodeContext { cf }
    }
}

impl Context for NodeContext {
    fn call<C: crate::RPC>(&self, req: C::Request) -> C::Response {
        todo!()
    }
}
