pub mod config;
pub mod docker;
pub mod logger;
pub mod project;
pub mod target;

macro_rules! fatal {
    ($($arg:tt)*) => {
        {
            ::log::error!($($arg)*);
            ::std::process::exit(1);
        }
    };
}

pub(crate) use fatal;

pub fn cli() -> clap::Command {
    use clap::{Arg, Command};

    Command::new("ammn")
        .about("CLI tool for Amimono projects")
        .arg(
            Arg::new("project")
                .short('p')
                .long("project")
                .help("Path to the project root. Defaults to the current directory."),
        )
        .subcommand_required(true)
        .subcommand(Command::new("run").about("Run a project locally."))
        .subcommand(Command::new("docker").about("Build docker image."))
        .subcommand(
            Command::new("deploy")
                .about("Deploy a project target.")
                .arg(
                    Arg::new("target")
                        .required(true)
                        .help("The target to deploy."),
                ),
        )
}

fn main() {
    logger::init();

    let matches = cli().get_matches();

    if let Some(x) = matches.get_one::<String>("project") {
        if let Err(e) = std::env::set_current_dir(x) {
            fatal!("could not find project {}: {}", x, e);
        }
    }

    let cf = config::load();
    let proj = project::get(&cf);

    match matches.subcommand() {
        Some(("run", _)) => project::run_local(&*proj),
        Some(("docker", _)) => docker::go(&*proj),
        Some(("deploy", sub)) => {
            let tgt_id = sub.get_one::<String>("target").unwrap();
            let tgt = target::get(&cf, &tgt_id);
            tgt.deploy(&*proj);
        }
        _ => unreachable!(),
    }
}
