use log::info;

use crate::{Application, Configuration, Location};

pub mod config;
pub mod cron;
pub mod ctx;
pub mod rpc;

pub use config::NodeConfig;
pub use ctx::NodeContext;

struct NodeLauncher {
    loc: Location,
    ctx: Option<NodeContext>,
}

impl NodeLauncher {
    pub fn new(cf: NodeConfig) -> NodeLauncher {
        NodeLauncher {
            loc: cf.location().clone(),
            ctx: Some(NodeContext::new(cf)),
        }
    }
}

impl Configuration for NodeLauncher {
    fn place_rpc<C: crate::RPC>(&mut self, _n_replicas: usize) {
        if C::LABEL == self.loc.0 {
            let ctx = self.ctx.take().unwrap();
            rpc::rpc_main::<C>(ctx);
        }
    }

    fn place_cron<C: crate::Cron>(&mut self) {
        if C::LABEL == self.loc.0 {
            let ctx = self.ctx.take().unwrap();
            cron::cron_main::<C>(ctx);
        }
    }
}

pub fn run_node<A: Application>(app: A) {
    let loc = match discover_location() {
        Ok(loc) => loc,
        Err(e) => panic!("run_node() could not discover location: {}", e),
    };

    info!("starting {:?}", loc);
    let cf = NodeConfig::new(loc, &app);
    let mut launcher = NodeLauncher::new(cf);
    app.setup(&mut launcher);
}

fn discover_location() -> Result<Location, &'static str> {
    let loc_env = match std::env::var("AMIMONO_LOCATION") {
        Ok(x) => x,
        Err(_) => return Err("AMIMONO_LOCATION not set"),
    };
    let (component, replica_str) = match loc_env.rsplit_once("-") {
        Some(x) => x,
        None => return Err("AMIMONO_LOCATION missing '-' before replica number"),
    };
    let replica = match usize::from_str_radix(replica_str, 10) {
        Ok(x) => x,
        Err(_) => return Err("could not parse replica number in AMIMONO_LOCATION"),
    };
    Ok(Location(component.to_owned(), replica))
}
