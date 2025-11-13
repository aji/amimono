#![feature(async_fn_traits)]

pub(crate) mod core;
pub(crate) mod local;
pub(crate) mod rpc;
pub mod toml;

pub use core::*;
pub use local::*;
use log::error;
pub use rpc::*;

pub fn entry(cf: AppConfig) {
    let job_env = std::env::var_os("AMIMONO_JOB").map(|s| s.into_string().unwrap());
    let bindings_file = std::env::var_os("AMIMONO_BINDINGS");

    let job_label = match job_env {
        Some(x) => x,
        None => {
            error!("AMIMONO_JOB not set");
            println!("This binary is not to be run directly.");
            println!("To run in local mode, use the Amimono CLI.");
            return;
        }
    };

    match job_label.as_str() {
        "_config" => {
            let out = ::toml::to_string(&cf.to_toml()).unwrap();
            print!("{}", out);
        }
        "_local" => {
            run_local(cf);
        }
        _ => {
            let bindings = match bindings_file {
                Some(x) => Bindings::from_file(&cf, x).expect("failed to load bindings"),
                None => panic!("AMIMONO_BINDINGS not set"),
            };
            job::run_job(&cf, &bindings, job_label.as_str());
        }
    }
}
