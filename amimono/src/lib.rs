pub(crate) mod core;
pub(crate) mod rpc;
pub mod runtime;
pub mod toml;

pub use core::*;
pub use rpc::*;
pub use runtime::Location;

use std::net::{Ipv4Addr, SocketAddr};

use crate::core::binding::BindingAllocator;

pub fn entry(cf: AppConfig) {
    let job_env = std::env::var_os("AMIMONO_JOB").map(|s| s.into_string().unwrap());
    let bindings_file = std::env::var_os("AMIMONO_BINDINGS");

    let job_label = match job_env {
        Some(x) => x,
        None => {
            log::error!("AMIMONO_JOB not set");
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
            let bindings = Bindings::new(&cf, LocalBindingAllocator::new());
            runtime::init(cf, bindings);
            job::run_all();
        }
        _ => {
            let bindings = match bindings_file {
                Some(x) => Bindings::from_file(&cf, x).expect("failed to load bindings"),
                None => panic!("AMIMONO_BINDINGS not set"),
            };
            runtime::init(cf, bindings);
            job::run_job(job_label.as_str());
        }
    }
}

struct LocalBindingAllocator {
    next_port: u16,
}

impl LocalBindingAllocator {
    fn new() -> LocalBindingAllocator {
        LocalBindingAllocator { next_port: 9000 }
    }
}

impl BindingAllocator for LocalBindingAllocator {
    fn next_http(&mut self, _job: &JobConfig) -> (SocketAddr, String) {
        let port = self.next_port;
        self.next_port += 1;
        let addr = (Ipv4Addr::LOCALHOST, port).into();
        let url = format!("http://localhost:{}", port);
        (addr, url)
    }
}
