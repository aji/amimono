use std::{
    marker::PhantomData,
    sync::{Arc, LazyLock},
};

use serde::{Deserialize, Serialize};

use crate::{
    config::{BindingType, ComponentConfig},
    runtime::{self, Component, ComponentRegistry},
};

pub trait RpcMessage: Serialize + for<'a> Deserialize<'a> + Send + 'static {
    fn verb(&self) -> &'static str;
}

pub trait Rpc: Sync + Send + 'static {
    type Request: RpcMessage;
    type Response: RpcMessage;

    fn start() -> Self;

    fn handle(&self, q: Self::Request) -> impl Future<Output = Self::Response> + Send;
}

pub struct RpcComponent<R>(PhantomData<R>);

impl<R: Rpc> Component for RpcComponent<R> {
    type Instance = Arc<R>;
}

impl<R: Rpc> RpcComponent<R> {
    fn register(reg: &mut ComponentRegistry, label: String) {
        reg.register::<Self>(label, Arc::new(R::start()))
    }

    fn entry() {
        let _instance = runtime::instance::<Self>().unwrap().clone();
        // TODO
    }

    fn component(label: String) -> ComponentConfig {
        ComponentConfig {
            label,
            binding: BindingType::Http,
            register: Self::register,
            entry: Self::entry,
        }
    }
}

pub fn component<R: Rpc>(label: String) -> ComponentConfig {
    RpcComponent::<R>::component(label)
}

#[derive(Debug, Clone)]
pub enum RpcError {
    Misc(String),
}

pub enum RpcClient<R> {
    Local(LazyLock<Arc<R>>),
}

impl<R: Rpc> Clone for RpcClient<R> {
    fn clone(&self) -> Self {
        RpcClient::<R>::new()
    }
}

impl<R: Rpc> RpcClient<R> {
    pub fn new() -> RpcClient<R> {
        RpcClient::Local(LazyLock::new(|| {
            runtime::instance::<RpcComponent<R>>()
                .expect("no local instance")
                .clone()
        }))
    }

    pub async fn call(&self, q: R::Request) -> Result<R::Response, RpcError> {
        match self {
            RpcClient::Local(instance) => Ok(instance.handle(q).await),
        }
    }

    pub fn local(&self) -> Option<&R> {
        match self {
            RpcClient::Local(instance) => Some(instance),
        }
    }
}

#[macro_export]
macro_rules! rpc_ops {
    {
        $(fn $op:ident ($($arg:ident: $arg_ty:ty),*) -> $ret_ty:ty;)*
    } => {
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[allow(non_camel_case_types)]
        pub enum Request {
            $($op($($arg_ty),*)),*
        }

        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[allow(non_camel_case_types)]
        pub enum Response {
            $($op($ret_ty)),*
        }

        impl ::amimono::rpc::RpcMessage for Request {
            fn verb(&self) -> &'static str {
                match self {
                    $( Request::$op(..) => stringify!($op) ),*
                }
            }
        }
        impl ::amimono::rpc::RpcMessage for Response {
            fn verb(&self) -> &'static str {
                match self {
                    $( Response::$op(..) => stringify!($op) ),*
                }
            }
        }

        pub trait Handler: Sync + Send + Sized + 'static {
            fn new() -> Self;

            $(fn $op(&self, $($arg: $arg_ty),*) -> impl Future<Output = $ret_ty> + Send;)*
        }

        pub struct Instance<H>(H);

        impl<H: Handler> ::amimono::rpc::Rpc for Instance<H> {
            type Request = Request;
            type Response = Response;

            fn start() -> Self {
                Instance(H::new())
            }

            async fn handle(&self, q: Request) -> Response {
                match q {
                    $(Request::$op($($arg),*) => {
                        let res = self.0.$op($($arg),*).await;
                        Response::$op(res)
                    })*
                }
            }
        }

        pub struct Client<H: Handler>(::amimono::rpc::RpcClient<Instance<H>>);

        impl<H: Handler> Clone for Client<H> {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }

        impl<H: Handler> Client<H> {
            pub fn new() -> Self {
                Client(::amimono::rpc::RpcClient::new())
            }

            $(
                pub async fn $op(&self, $($arg: $arg_ty),*)
                -> Result<$ret_ty, ::amimono::rpc::RpcError> {
                    use ::amimono::rpc::RpcMessage;
                    if let Some(local) = self.0.local() {
                        Ok(local.0.$op($($arg),*).await)
                    } else {
                        let q = Request::$op($($arg),*);
                        match self.0.call(q).await {
                            Ok(Response::$op(a)) => Ok(a),
                            Ok(x) => panic!("got {} but was expecting {}", x.verb(), stringify!($op)),
                            Err(e) => Err(e)
                        }
                    }
                }
            )*
        }

        pub fn component<H: Handler>(label: String) -> ::amimono::config::ComponentConfig {
            ::amimono::rpc::component::<Instance<H>>(label)
        }
    }
}
