pub mod config;
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
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::SetTrue)
                .help("Enable verbose logging."),
        )
        .subcommand_required(true)
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
    let matches = cli().get_matches();

    logger::init(matches.get_flag("verbose"));

    if let Some(x) = matches.get_one::<String>("project") {
        if let Err(e) = std::env::set_current_dir(x) {
            fatal!("could not find project {}: {}", x, e);
        }
    }

    let cf = config::load();
    let proj = project::Project::from_config(&cf);

    match matches.subcommand() {
        Some(("deploy", sub_m)) => {
            let target_name = sub_m
                .get_one::<String>("target")
                .expect("target is required");
            let target = target::Target::from_config(&cf, target_name);
            target.deploy(&proj);
        }
        _ => unreachable!("subcommand is required"),
    }
}
