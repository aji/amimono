pub struct Args {
    pub action: Action,
    pub bind: Option<String>,
    pub r#static: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    DumpConfig,
    Local,
    Job(String),
}

pub fn parse_args() -> Result<Args, String> {
    use clap::{Arg, ArgAction, Command};

    let m = Command::new("amimono")
        .arg(
            Arg::new("dump-config")
                .long("dump-config")
                .action(ArgAction::SetTrue)
                .help("Dump the application configuration and exit"),
        )
        .arg(
            Arg::new("local")
                .long("local")
                .action(ArgAction::SetTrue)
                .help("Run in local mode"),
        )
        .arg(
            Arg::new("job")
                .long("job")
                .action(ArgAction::Set)
                .help("The job to run"),
        )
        .arg(
            Arg::new("static")
                .long("static")
                .action(ArgAction::Set)
                .help("The static config root to use. Forces the static runtime."),
        )
        .arg(
            Arg::new("bind")
                .long("bind")
                .action(ArgAction::Set)
                .help("The IP address to bind to."),
        )
        .get_matches();

    let action = [
        m.get_flag("dump-config").then_some(Action::DumpConfig),
        m.get_flag("local").then_some(Action::Local),
        m.get_one::<String>("job").map(|j| Action::Job(j.clone())),
    ]
    .into_iter()
    .filter(|x| x.is_some())
    .reduce(|_, _| None)
    .flatten()
    .ok_or("must specify exactly one of --local, --job <job>, or --dump-config")?;

    let bind = m.get_one::<String>("bind").cloned();
    let r#static = m.get_one::<String>("static").cloned();

    Ok(Args {
        action,
        bind,
        r#static,
    })
}
