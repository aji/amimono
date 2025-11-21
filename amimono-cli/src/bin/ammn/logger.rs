use colored::Colorize;

struct Logger;
const LOGGER: Logger = Logger;

fn message(rec: &log::Record) -> String {
    let module = rec.module_path().unwrap_or("unknown");
    let prefix = match rec.level() {
        log::Level::Error => format!("{}", format!("ERROR [{}] ", module).bright_red().bold()),
        log::Level::Warn => format!("{}", "WARN ".bright_yellow().bold()),
        _ => "".to_string(),
    };
    format!("{}{}", prefix, rec.args())
}

impl log::Log for Logger {
    fn enabled(&self, md: &log::Metadata) -> bool {
        md.level() <= log::Level::Info
    }

    fn log(&self, rec: &log::Record) {
        if self.enabled(rec.metadata()) {
            eprintln!("{:>12} {}", "Amimono".magenta().bold(), message(rec));
        }
    }

    fn flush(&self) {}
}

pub fn init() {
    log::set_logger(&LOGGER).expect("could not initialize logger");
    log::set_max_level(log::LevelFilter::Info);
}
