use std::net::SocketAddr;

#[derive(Debug)]
pub enum BindingType {
    None,
    TCP(usize),
}

#[derive(Debug)]
pub enum LocalBinding {
    None,
    TCP(Vec<SocketAddr>),
}

#[derive(Debug)]
pub enum RemoteBinding {
    None,
    TCP(Vec<SocketAddr>),
}
