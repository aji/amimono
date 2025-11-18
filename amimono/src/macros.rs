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
                    $(Request::$op(..) => stringify!($op)),*
                }
            }
        }
        impl ::amimono::rpc::RpcMessage for Response {
            fn verb(&self) -> &'static str {
                match self {
                    $(Response::$op(..) => stringify!($op)),*
                }
            }
        }

        pub trait Handler: Sync + Send + Sized + 'static {
            fn new() -> impl Future<Output = Self> + Send;

            $(fn $op(&self, $($arg: $arg_ty),*) -> impl Future<Output = $ret_ty> + Send;)*
        }

        pub struct Instance<H>(H);

        impl<H: Handler> ::amimono::rpc::Rpc for Instance<H> {
            type Request = Request;
            type Response = Response;

            async fn start() -> Self {
                Instance(H::new().await)
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

            $(pub async fn $op(&self, $($arg: $arg_ty),*)
            -> Result<$ret_ty, ::amimono::rpc::RpcError> {
                use ::amimono::rpc::RpcMessage;

                if let Some(local) = self.0.local().await {
                    return Ok(local.0.$op($($arg),*).await);
                }

                let q = Request::$op($($arg),*);
                match self.0.call(q).await {
                    Ok(Response::$op(a)) => Ok(a),
                    Ok(x) => panic!("got {} but was expecting {}", x.verb(), stringify!($op)),
                    Err(e) => Err(e)
                }
            })*
        }

        pub fn component<H: Handler>(label: String) -> ::amimono::config::ComponentConfig {
            ::amimono::rpc::component::<Instance<H>>(label)
        }
    }
}
