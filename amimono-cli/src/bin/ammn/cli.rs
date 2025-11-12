use clap::{Arg, Command};

pub fn cli() -> Command {
    Command::new("ammn")
        .about("CLI tool for amimono projects")
        .arg(
            Arg::new("project")
                .short('p')
                .long("project")
                .help("Path to the project root. Defaults to the current directory."),
        )
        .subcommand(
            Command::new("run")
                .about("Run a project target.")
                .arg(Arg::new("target").required(true).help("The target to run.")),
        )
}
