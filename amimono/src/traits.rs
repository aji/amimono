use std::time::Duration;

pub trait Context {
    fn call<C: RPC>(&self, req: C::Request) -> C::Response;
}

pub trait Component: Sized {
    const LABEL: &'static str;

    fn init() -> Self;
}

pub trait RPC: Component + Send + Sync + 'static {
    type Request: serde::Serialize + for<'a> serde::Deserialize<'a>;
    type Response: serde::Serialize + for<'a> serde::Deserialize<'a>;

    fn handle<X: Context>(&self, ctx: &X, req: Self::Request) -> Self::Response;

    fn place<Cf: Configuration>(cf: &mut Cf, n: usize) {
        cf.place_rpc::<Self>(n);
    }
    fn call<X: Context>(ctx: &X, req: Self::Request) -> Self::Response {
        ctx.call::<Self>(req)
    }
}

pub trait Cron: Component {
    const INTERVAL: Duration;
    fn fire<X: Context>(&self, ctx: &X);

    fn place<Cf: Configuration>(cf: &mut Cf) {
        cf.place_cron::<Self>();
    }
}

pub trait Configuration {
    fn place_rpc<C: RPC>(&mut self, n_replicas: usize);
    fn place_cron<C: Cron>(&mut self);
}

pub trait Application: Sized {
    fn setup<Cf: Configuration>(&self, cf: &mut Cf);
}
