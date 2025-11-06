use std::collections::HashMap;

use crate::{Application, Configuration, Location};

pub enum NodePlacement {
    RPC(usize),
    Cron,
}

pub struct NodeConfig {
    loc: Location,
    placements: HashMap<String, NodePlacement>,
}

impl NodeConfig {
    pub fn new<A: Application>(loc: Location, app: &A) -> NodeConfig {
        let mut cf = NodeConfig {
            loc,
            placements: HashMap::new(),
        };
        app.setup(&mut cf);
        cf
    }

    pub fn location(&self) -> &Location {
        &self.loc
    }

    fn place(&mut self, k: &'static str, v: NodePlacement) {
        let is_none = self.placements.insert(k.to_owned(), v).is_none();
        assert!(is_none);
    }
}

impl Configuration for NodeConfig {
    fn place_rpc<C: crate::RPC>(&mut self, n_replicas: usize) {
        self.place(C::LABEL, NodePlacement::RPC(n_replicas));
    }

    fn place_cron<C: crate::Cron>(&mut self) {
        self.place(C::LABEL, NodePlacement::Cron);
    }
}
