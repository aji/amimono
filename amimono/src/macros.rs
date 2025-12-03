/// A helper macro for defining RPC components.
///
/// This is the recommended way to define an RPC component since it
/// significantly reduces boilerplate, especially for RPC components that have
/// more than 1 method. The macro is invoked with a series of `fn` definitions
/// that represent operations. All parameter and return types must be fully
/// serializable and deserializable via serde.
///
/// # Example
///
/// ```
/// mod ops {
///     amimono::rpc_ops! {
///         fn add_item(key: String, value: String) -> ();
///         fn get_item(key: String) -> Option<String>;
///         fn delete_item(key: String) -> ();
///     }
/// }
///
/// pub struct MapService;
///
/// pub type MapClient = ops::Client<MapService>;
///
/// impl ops::Handler for MapService {
///     async fn new() -> Self {
///         // Other initialization such as creating clients can be done here,
///         // although be careful to avoid deadlocks if making RPC calls during
///         // initialization.
///         MapService
///     }
///
///     async fn add_item(&self, key: String, value: String) -> () {
///         // ...
///     }
///     async fn get_item(&self, key: String) -> Option<String> {
///         // ...
///     }
///     async fn delete_item(&self, key: String) -> () {
///         // ...
///     }
/// }
/// ```
///
/// The `MapClient` alias above has an `impl` that behaves like the following:
///
/// ```
/// use amimono::rpc::RpcResult;
///
/// impl MapClient {
///     pub fn new() -> MapClient;
///
///     pub async fn add_item(&self, key: String, value: String) -> RpcResult<()>;
///     pub async fn get_item(&self, key: String) -> RpcResult<Option<String>>;
///     pub async fn delete_item(&self, key: String) -> RpcResult<()>;
/// }
/// ```
///
/// A `ComponentConfig` can be created as follows:
///
/// ```
/// use amimono::config::ComponentConfig;
///
/// pub fn component() -> ComponentConfig {
///     ops::component::<MapService>("mapservice".to_owned());
/// }
/// ```
///
/// For a working example, refer to any of the Amimono example projects.
#[macro_export]
macro_rules! rpc_ops {
    {
        const LABEL: &'static str = $label:expr;

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

            $(fn $op(&self, $($arg: $arg_ty),*)
            -> impl Future<Output = ::amimono::rpc::RpcResult<$ret_ty>> + Send;)*
        }

        pub struct Instance<H>(H);

        impl<H: Handler> ::amimono::rpc::Rpc for Instance<H> {
            type Request = Request;
            type Response = Response;

            const LABEL: &'static str = $label;

            async fn start() -> Self {
                Instance(H::new().await)
            }

            async fn handle(&self, q: Request)
            -> ::amimono::rpc::RpcResult<Response> {
                match q {
                    $(Request::$op($($arg),*) => {
                        match self.0.$op($($arg),*).await {
                            Ok(res) => Ok(Response::$op(res)),
                            Err(e) => Err(e),
                        }
                    })*
                }
            }
        }

        pub type Component<H> = ::amimono::rpc::RpcComponent<Instance<H>>;

        pub type ComponentImpl<H> = ::amimono::rpc::RpcComponentImpl<Instance<H>>;

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

            pub fn at(&self, loc: ::amimono::component::Location) -> ClientAt<H> {
                ClientAt {
                    loc,
                    inner: self.0.clone(),
                }
            }

            $(pub async fn $op(&self, $($arg: $arg_ty),*)
            -> ::amimono::rpc::RpcResult<$ret_ty> {
                use ::amimono::rpc::RpcMessage;

                if let Some(local) = self.0.local().await {
                    return local.0.$op($($arg),*).await;
                }

                let q = Request::$op($($arg),*);
                match self.0.call(q).await {
                    Ok(Response::$op(a)) => Ok(a),
                    Ok(x) => panic!("got {} but was expecting {}", x.verb(), stringify!($op)),
                    Err(e) => Err(e)
                }
            })*
        }

        pub struct ClientAt<H: Handler> {
            loc: ::amimono::component::Location,
            inner: ::amimono::rpc::RpcClient<Instance<H>>,
        }

        impl<H: Handler> Clone for ClientAt<H> {
            fn clone(&self) -> Self {
                Self {
                    loc: self.loc.clone(),
                    inner: self.inner.clone(),
                }
            }
        }

        impl<H: Handler> ClientAt<H> {
            $(pub async fn $op(&self, $($arg: $arg_ty),*)
            -> ::amimono::rpc::RpcResult<$ret_ty> {
                use ::amimono::rpc::RpcMessage;

                let q = Request::$op($($arg),*);
                match self.inner.call_at(self.loc.clone(), q).await {
                    Ok(Response::$op(a)) => Ok(a),
                    Ok(x) => panic!("got {} but was expecting {}", x.verb(), stringify!($op)),
                    Err(e) => Err(e)
                }
            })*
        }
    }
}
