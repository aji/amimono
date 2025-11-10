use std::net::SocketAddr;

pub trait BindingAllocator {
    fn next_http(&mut self) -> (SocketAddr, String);
}
