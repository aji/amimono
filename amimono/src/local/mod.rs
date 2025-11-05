use std::{
    collections::HashMap,
    sync::{Arc, mpsc},
    thread,
};

use rand::Rng;

use crate::{Location, traits::*};

enum LocalPlacement {
    RPC(usize),
    Cron,
}

struct LocalConfig {
    placements: HashMap<String, LocalPlacement>,
}

impl LocalConfig {
    fn new<A: Application>(app: &A) -> LocalConfig {
        let mut cf = LocalConfig {
            placements: HashMap::new(),
        };
        app.setup(&mut cf);
        cf
    }
}
impl Configuration for LocalConfig {
    fn place_rpc<C: RPC>(&mut self, n: usize) {
        let is_none = self
            .placements
            .insert(C::LABEL.to_owned(), LocalPlacement::RPC(n))
            .is_none();
        assert!(is_none);
    }
    fn place_cron<C: Cron>(&mut self) {
        let is_none = self
            .placements
            .insert(C::LABEL.to_owned(), LocalPlacement::Cron)
            .is_none();
        assert!(is_none);
    }
}

struct LocalLauncher {
    cf: Arc<LocalConfig>,
    threads: Vec<thread::JoinHandle<()>>,
    rpc: LocalRPCManifold,
}

impl LocalLauncher {
    fn new(cf: Arc<LocalConfig>) -> LocalLauncher {
        let rpc = LocalRPCManifold::new(&*cf);
        let threads = Vec::new();
        LocalLauncher { cf, threads, rpc }
    }
    fn run<A: Application>(mut self, app: &A) {
        app.setup(&mut self);
        for thread in self.threads {
            thread.join().unwrap();
        }
    }
    fn make_context(&self, loc: Location) -> LocalContext {
        LocalContext {
            loc,
            cf: self.cf.clone(),
            rpc: self.rpc.clone_senders(),
        }
    }
}
impl Configuration for LocalLauncher {
    fn place_rpc<C: RPC>(&mut self, n: usize) {
        for i in 0..n {
            let ctx = self.make_context(Location(C::LABEL.to_owned(), i));
            let rx = self.rpc.take_receiver(&ctx.loc);
            self.threads.push(LocalRPC::spawn::<C>(ctx, rx));
        }
    }
    fn place_cron<C: Cron>(&mut self) {
        let ctx = self.make_context(Location(C::LABEL.to_owned(), 0));
        self.threads.push(LocalCron::spawn::<C>(ctx));
    }
}

struct LocalContext {
    loc: Location,
    cf: Arc<LocalConfig>,
    rpc: HashMap<Location, LocalRPCSender>,
}

impl Context for LocalContext {
    fn call<C: RPC>(&self, q: C::Request) -> C::Response {
        let (tx, rx) = mpsc::channel();
        let loc = match self.cf.placements.get(C::LABEL).unwrap() {
            LocalPlacement::RPC(n) => {
                Location(C::LABEL.to_owned(), rand::rng().random_range(0..*n))
            }
            _ => panic!(),
        };
        let q_str = serde_json::to_string(&q).unwrap();
        self.rpc.get(&loc).unwrap().send((q_str, tx)).unwrap();
        let a_str = rx.recv().unwrap();
        serde_json::from_str(&a_str).unwrap()
    }
}

type LocalRPCSender = mpsc::Sender<(String, mpsc::Sender<String>)>;
type LocalRPCReceiver = mpsc::Receiver<(String, mpsc::Sender<String>)>;

struct LocalRPCManifold {
    senders: HashMap<Location, LocalRPCSender>,
    receivers: HashMap<Location, LocalRPCReceiver>,
}

impl LocalRPCManifold {
    fn new(cf: &LocalConfig) -> LocalRPCManifold {
        let mut senders = HashMap::new();
        let mut receivers = HashMap::new();
        for (label, placement) in cf.placements.iter() {
            if let LocalPlacement::RPC(n) = placement {
                for i in 0..*n {
                    let loc = Location(label.clone(), i);
                    let (tx, rx) = mpsc::channel();
                    senders.insert(loc.clone(), tx);
                    receivers.insert(loc.clone(), rx);
                }
            }
        }
        LocalRPCManifold { senders, receivers }
    }

    fn clone_senders(&self) -> HashMap<Location, LocalRPCSender> {
        self.senders.clone()
    }

    fn take_receiver(&mut self, loc: &Location) -> LocalRPCReceiver {
        self.receivers.remove(loc).unwrap()
    }
}

struct LocalRPC {
    ctx: LocalContext,
}
impl LocalRPC {
    fn spawn<C: RPC>(ctx: LocalContext, rpc: LocalRPCReceiver) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let job = LocalRPC { ctx };
            job.main::<C>(rpc);
        })
    }
    fn main<C: RPC>(self, rpc: LocalRPCReceiver) {
        println!("{:?} rpc start!", self.ctx.loc);
        let job = C::init();
        loop {
            let (q_in, a_out) = rpc.recv().unwrap();
            let q = serde_json::from_str(&q_in).unwrap();
            println!("{:?} got {:?}", self.ctx.loc, q_in);
            let a = serde_json::to_string(&job.handle(&self.ctx, q)).unwrap();
            a_out.send(a).unwrap();
        }
    }
}

struct LocalCron {
    ctx: LocalContext,
}
impl LocalCron {
    fn spawn<C: Cron>(ctx: LocalContext) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let job = LocalCron { ctx };
            job.main::<C>();
        })
    }
    fn main<C: Cron>(self) {
        println!("{:?} cron start!", self.ctx.loc);
        let job = C::init();
        loop {
            println!("{:?} cron fire!", self.ctx.loc);
            job.fire(&self.ctx);
            thread::sleep(C::INTERVAL);
        }
    }
}

pub fn run_local<A: Application>(app: A) {
    let cf = Arc::new(LocalConfig::new(&app));
    let launcher = LocalLauncher::new(cf);
    launcher.run(&app);
}
