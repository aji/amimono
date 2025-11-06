use clap::Command;

use crate::Application;

fn cli<A: Application>() -> Command {
    Command::new("amimono")
        .about(format!("Main {} binary.", A::LABEL))
        .subcommand_required(true)
        .subcommand(
            Command::new("local")
                .about("Launch the full application in memory, e.g. for local development."),
        )
        .subcommand(
            Command::new("node")
                .about("Start the configuration for this node. (Internal command.)"),
        )
}

pub fn main<A: Application>(app: A) {
    let matches = cli::<A>().get_matches();

    match matches.subcommand() {
        Some(("local", _)) => crate::local::run_local(app),
        Some(("node", _)) => crate::node::run_node(app),
        _ => unreachable!(),
    }
}
