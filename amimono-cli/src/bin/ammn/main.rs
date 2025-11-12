use crate::config::Config;

pub mod cli;
pub mod config;

fn main() {
    let matches = cli::cli().get_matches();

    println!("{:#?}", matches);

    let cf_file =
        std::fs::read_to_string("amimono.toml").expect("no amimono.toml in current directory");
    let cf: Config = toml::de::from_str(&cf_file).expect("could not parse amimono.toml");

    println!("{:#?}", cf);
}
