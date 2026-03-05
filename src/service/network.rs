use core::net::SocketAddr;
use std::net::{TcpListener, TcpStream};

pub struct HostedNetwork {
    // 跟踪宿主监听器与连接
}

impl HostedNetwork {
    pub fn new() -> Self {
        Self {}
    }

    pub fn listen(&self, addr: SocketAddr) -> std::io::Result<TcpListener> {
        TcpListener::bind(addr)
    }

    pub fn connect(&self, addr: SocketAddr) -> std::io::Result<TcpStream> {
        TcpStream::connect(addr)
    }
}
