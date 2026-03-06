use std::net::{TcpListener, TcpStream, UdpSocket, SocketAddr};
use std::collections::HashMap;
use std::sync::Mutex;
use std::io::{Read, Write};

pub struct NetworkManager {
    listeners: Mutex<HashMap<usize, TcpListener>>,
    streams: Mutex<HashMap<usize, TcpStream>>,
    udp_sockets: Mutex<HashMap<usize, UdpSocket>>,
    next_handle: Mutex<usize>,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            listeners: Mutex::new(HashMap::new()),
            streams: Mutex::new(HashMap::new()),
            udp_sockets: Mutex::new(HashMap::new()),
            next_handle: Mutex::new(1),
        }
    }

    pub fn listen(&self, addr: SocketAddr) -> Result<usize, std::io::Error> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        
        let mut handle = self.next_handle.lock().unwrap();
        let h = *handle;
        *handle += 1;
        
        self.listeners.lock().unwrap().insert(h, listener);
        Ok(h)
    }

    pub fn connect(&self, addr: SocketAddr) -> Result<usize, std::io::Error> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nonblocking(true)?;

        let mut handle = self.next_handle.lock().unwrap();
        let h = *handle;
        *handle += 1;
        
        self.streams.lock().unwrap().insert(h, stream);
        Ok(h)
    }

    pub fn close(&self, handle: usize) {
        self.listeners.lock().unwrap().remove(&handle);
        self.streams.lock().unwrap().remove(&handle);
        self.udp_sockets.lock().unwrap().remove(&handle);
    }

    pub fn send(&self, handle: usize, buf: &[u8]) -> Result<usize, std::io::Error> {
        if let Some(stream) = self.streams.lock().unwrap().get_mut(&handle) {
            stream.write(buf)
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Stream not found"))
        }
    }

    pub fn recv(&self, handle: usize, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        if let Some(stream) = self.streams.lock().unwrap().get_mut(&handle) {
            stream.read(buf)
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Stream not found"))
        }
    }
}
