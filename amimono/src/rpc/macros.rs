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

        impl Response {
            fn verb(&self) -> &'static str {
                match self {
                    $(Response::$op(_) => stringify!($op)),*
                }
            }
        }

        pub trait Handler: Sync + Send + Sized + 'static {
            const LABEL: ::amimono::Label;

            fn new() -> impl Future<Output = Self> + Send;

            $(
                fn $op(&self, $($arg: $arg_ty),*)
                -> impl Future<Output = $ret_ty> + Send;
            )*
        }

        pub struct RpcHandler<H>(H);

        impl<H> From<H> for RpcHandler<H> {
            fn from(other: H) -> RpcHandler<H> {
                RpcHandler(other)
            }
        }

        impl<H: Handler> ::amimono::RpcHandler for RpcHandler<H> {
            type Request = Request;
            type Response = Response;

            async fn handle(&self, q: Self::Request) -> Self::Response {
                match q {
                    $(
                        Request::$op($($arg),*) => {
                            let res = self.0.$op($($arg),*).await;
                            Response::$op(res)
                        }
                    )*
                }
            }
        }

        impl<H: Handler> ::amimono::Rpc for RpcHandler<H> {
            const LABEL: ::amimono::Label = H::LABEL;

            type Handler = Self;

            async fn start() -> Self::Handler {
                RpcHandler(H::new().await)
            }
        }

        pub struct RpcClient<H: Handler>(::amimono::RpcClient<RpcHandler<H>>);

        impl<H: Handler> Clone for RpcClient<H> {
            fn clone(&self) -> Self {
                RpcClient(self.0.clone())
            }
        }

        impl<H: Handler> RpcClient<H> {
            pub async fn new() -> Self {
                RpcClient(::amimono::RpcClient::new().await)
            }

            $(
                pub async fn $op(&self, $($arg: $arg_ty),*)
                -> Result<$ret_ty, ::amimono::RpcError> {
                    let q = Request::$op($($arg),*);
                    match self.0.call(q).await {
                        Ok(Response::$op(a)) => Ok(a),
                        Ok(x) => panic!("got {} but was expecting {}", x.verb(), stringify!($op)),
                        Err(e) => Err(e)
                    }
                }
            )*
        }

        pub fn component<H: Handler>() -> ::amimono::Component {
            <RpcHandler::<H> as ::amimono::Rpc>::component()
        }
    }
}
