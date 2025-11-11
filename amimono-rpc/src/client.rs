use std::marker::PhantomData;

use amimono::{Label, Runtime};
use reqwest::{Client, Url};

use crate::Rpc;

pub struct RpcClientBuilder {
    rt: Runtime,
    client: Client,
}

pub struct RpcClient<C> {
    rt: Runtime,
    target: Label,
    phantom: PhantomData<C>,
    // target: PhantomData<C>,
    // endpoint: Url,
    // client: Client,
}

impl RpcClientBuilder {
    pub fn new(rt: &Runtime) -> RpcClientBuilder {
        RpcClientBuilder {
            rt: rt.clone(),
            client: Client::new(),
        }
    }

    pub fn get<C: Rpc>(&self) -> RpcClient<C> {
        //let endpoint: &str = get_endpoint();

        RpcClient {
            rt: self.rt.clone(),
            target: C::LABEL,
            phantom: PhantomData,
            // target: PhantomData,
            // endpoint: Url::parse(&endpoint).unwrap(),
            // client: self.client.clone(),
        }
    }
}

impl<C: Rpc> RpcClient<C> {
    pub async fn call(&self, req: C::Request) -> C::Response {
        self.rt.call_local(self.target, req).await
        /*
        self.client
            .post(self.endpoint.clone())
            .json(&req)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
            */
    }
}

fn get_endpoint() -> &'static str {
    todo!()
}
