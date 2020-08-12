use std::net::{SocketAddr, UdpSocket};
use crate::transport::connection::*;
use crate::transport::frame::*;
use crate::transport::common::default_host_info;
use log;

pub struct Listener {
    socket: UdpSocket,
}

impl Listener {
    pub fn new(bind: SocketAddr) -> Self {
        let socket: UdpSocket = UdpSocket::bind(bind).expect("Could not bind to Socket.");
        socket.set_nonblocking(true).unwrap();
        Self {socket: Some(socket)}
    }

    /// Used by servers to listen for incoming connections.
    /// non-blocking.
    pub fn listen_once(&mut self, accept_callback: Box<ObjectListener>, timeout_callback: Box<TimeoutListener>) -> Option<Connection> {
        // try to consume the socket to transfer its ownership to Connection
        if let Some(socket) = &mut self.socket {
            let mut buf: [u8; 10] = [0; 10];
            if let Ok((_n_bytes, src)) = socket.peek_from(&mut buf) {
                // heureka! We got a client!
                let socket = self.socket.take().unwrap(); // consume known existing socket
                socket.connect(src).unwrap();
                let conn = Connection {
                    send_jobs: Vec::new(),
                    recv_jobs: Vec::new(),
                    accept_callback,
                    timeout_callback,
                    socket,
                    dest: src,
                    is_server: true,
                    self_info: default_host_info(),
                    peer_info: None,
                    session: None,
                };
                return Some(conn);
            };
            return None;
        }

        // socket already consumed.
        log::warn!("Listening on dead Listener. It is dead because it has already produced a connection. If the connection is completed, instanciate a new listener. ");
        return None;
    }
}
