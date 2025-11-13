#![feature(async_fn_traits)]

pub(crate) mod core;
pub(crate) mod local;
pub(crate) mod rpc;
pub mod toml;

pub use core::*;
pub use local::*;
pub use rpc::*;
use std::sync::Arc;

pub fn entry(cf: AppConfig) {
    let job = std::env::var_os("AMIMONO_JOB").map(|s| s.into_string().unwrap());
    let bindings_file = std::env::var_os("AMIMONO_BINDINGS");

    if let Some(job_label) = job {
        match job_label.as_str() {
            "_config" => println!("dump config"),
            _ => {
                let bindings = match bindings_file {
                    Some(x) => Bindings::from_file(&cf, x).expect("failed to load bindings"),
                    None => panic!("AMIMONO_BINDINGS not set"),
                };
                job::run_job(&cf, Arc::new(bindings), job_label.as_str());
            }
        }
    } else {
        let out = ::toml::to_string(&cf.to_toml()).unwrap();
        print!("{}", out);
    }
}
