#[derive(Debug)]
pub enum BindingType {
    None,
    TCP(usize),
}

#[derive(Debug)]
pub enum LocalBinding {
    None,
    TCP(Vec<u16>),
}

#[derive(Debug)]
pub enum RemoteBinding {
    None,
    TCP(Vec<(String, u16)>),
}
