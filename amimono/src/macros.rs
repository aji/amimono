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
///         const LABEL: &'static str = "mapservice";
///
///         fn add_item(key: String, value: String) -> ();
///         fn get_item(key: String) -> Option<String>;
///         fn delete_item(key: String) -> ();
///     }
/// }
///
/// pub struct MapService;
///
/// pub type MapClient = ops::Client;
/// pub type MapComponentKind = ops::ComponentKind;
/// pub type MapComponent = ops::Component<MapService>;
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
/// The component can be installed in an `AppConfig` as follows, using the
/// `MapComponent` alias defined above:
///
/// ```
/// use amimono::config::AppBuilder;
///
/// pub fn install(app: &mut AppBuilder) {
///     app.add_job(
///         JobBuilder::new()
///             .with_label("mapservice")
///             .install(MapComponent::installer)
///     );
/// }
/// ```
///
/// For a working example, refer to any of the Amimono example projects.
#[macro_export]
macro_rules! rpc_component {
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

            $(fn $op(&self, $($arg: &$arg_ty),*)
            -> impl Future<Output = ::amimono::rpc::RpcResult<$ret_ty>> + Send;)*
        }

        trait BoxHandler: Sync + Send + 'static {
            $(fn $op<'s: 'f, 'r: 'f, 'f>(&'s self, $($arg: &'r $arg_ty),*)
            -> ::futures::future::BoxFuture<'f, ::amimono::rpc::RpcResult<$ret_ty>>;)*
        }

        impl<H: Handler> BoxHandler for H {
            $(fn $op<'s: 'f, 'r: 'f, 'f>(&'s self, $($arg: &'r $arg_ty),*)
            -> ::futures::future::BoxFuture<'f, ::amimono::rpc::RpcResult<$ret_ty>> {
                Box::pin(<Self as Handler>::$op(self, $($arg),*))
            })*
        }

        pub struct ComponentKind;

        impl ::amimono::rpc::RpcComponentKind for ComponentKind {
            type Request = Request;
            type Response = Response;

            const LABEL: &'static str = $label;
        }

        pub struct Component<H>(H);

        impl<H: Handler> ::amimono::rpc::RpcComponent for Component<H> {
            type Kind = ComponentKind;

            async fn start() -> Self {
                Component(H::new().await)
            }

            async fn handle(&self, q: &Request)
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

        pub struct Client<R = ::amimono::retry::Retry>(::amimono::rpc::RpcClient<ComponentKind, R>);

        impl<R: Clone> Clone for Client<R> {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }

        impl Default for Client {
            fn default() -> Self {
                Self::new()
            }
        }

        impl Client {
            pub fn new() -> Self {
                Client(::amimono::rpc::RpcClient::new())
            }
        }

        impl<R: Sync> Client<R> {
            pub fn with_retry<X>(self, retry: X) -> Client<X> {
                Client(self.0.with_retry(retry))
            }
        }

        impl<R: Clone> Client<R> {
            pub fn at<A>(&self, loc: ::amimono::component::Location<A>) -> ClientAt<A, R> {
                ClientAt {
                    loc,
                    inner: self.0.clone(),
                }
            }
        }

        impl<R: ::amimono::retry::RetryStrategy<::amimono::rpc::RpcError>> Client<R> {
            $(pub async fn $op(&self, $($arg: $arg_ty),*)
            -> ::amimono::rpc::RpcResult<$ret_ty> {
                use ::amimono::rpc::RpcMessage;

                let q = Request::$op($($arg),*);
                match self.0.call(&q).await {
                    Ok(Response::$op(a)) => Ok(a),
                    Ok(x) => panic!("got {} but was expecting {}", x.verb(), stringify!($op)),
                    Err(e) => Err(e)
                }
            })*
        }

        pub struct ClientAt<A, R = ::amimono::retry::Retry> {
            loc: ::amimono::component::Location<A>,
            inner: ::amimono::rpc::RpcClient<ComponentKind, R>,
        }

        impl<A, R: Sync> ClientAt<A, R> {
            pub fn with_retry<X>(self, retry: X) -> ClientAt<A, X> {
                ClientAt {
                    loc: self.loc,
                    inner: self.inner.with_retry(retry)
                }
            }
        }

        impl<A: Clone, R: Clone> Clone for ClientAt<A, R> {
            fn clone(&self) -> Self {
                Self {
                    loc: self.loc.clone(),
                    inner: self.inner.clone(),
                }
            }
        }

        impl<A> ClientAt<A> where A: ::std::borrow::Borrow<str> {
            $(pub async fn $op(&self, $($arg: $arg_ty),*)
            -> ::amimono::rpc::RpcResult<$ret_ty> {
                use ::amimono::rpc::RpcMessage;

                let q = Request::$op($($arg),*);
                match self.inner.call_at(&self.loc, &q).await {
                    Ok(Response::$op(a)) => Ok(a),
                    Ok(x) => panic!("got {} but was expecting {}", x.verb(), stringify!($op)),
                    Err(e) => Err(e)
                }
            })*
        }
    }
}
