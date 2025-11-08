use std::marker::PhantomData;

use amimono::{Context, RemoteBinding};
use reqwest::{Url, blocking::Client};

use crate::{Rpc, component::RpcComponent};

pub struct RpcClientBuilder<'a, X> {
    ctx: &'a X,
    client: Client,
}

pub struct RpcClient<C> {
    target: PhantomData<C>,
    endpoint: Url,
    client: Client,
}

impl<'a, X: Context> RpcClientBuilder<'a, X> {
    pub fn new(ctx: &'a X) -> RpcClientBuilder<'a, X> {
        RpcClientBuilder {
            ctx,
            client: Client::new(),
        }
    }

    pub fn get<C: Rpc>(&self) -> RpcClient<C> {
        let endpoint = match self.ctx.locate::<RpcComponent<C>>() {
            RemoteBinding::None => panic!(),
            RemoteBinding::TCP(addrs) => format!("http://{}/rpc", addrs[0]),
        };

        RpcClient {
            target: PhantomData,
            endpoint: Url::parse(&endpoint).unwrap(),
            client: self.client.clone(),
        }
    }
}

impl<C: Rpc> RpcClient<C> {
    pub fn call(&self, req: C::Request) -> C::Response {
        self.client
            .post(self.endpoint.clone())
            .json(&req)
            .send()
            .unwrap()
            .json()
            .unwrap()
    }
}
