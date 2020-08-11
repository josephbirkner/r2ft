use std::net::{SocketAddr, UdpSocket};
use crate::common::udp::Socket;
use crate::transport::connection::*;

/// Is called by the transport layer to inform the application
/// about a new connection coming from `incoming`. The server-
/// application then uses that SocketAddr to get a `Connection`
/// via `crate::transport::client::connect()`.
/// TODO: fn(incoming: SocketAddr) and then app shall call connect on that addr
pub type ConnectionListener = fn (incoming: SocketAddr) -> ();

pub struct Listener {
    socket: UdpSocket,
    callback: ConnectionListener,
}

impl Listener {
    /// Used by servers to listen for incoming connections.
    /// Install a ConnectionListener to be called for each new
    /// incoming connection. Will listen at `bind`.
    /// Non-blocking.
    pub fn new(bind: SocketAddr, callback: ConnectionListener) -> Listener {
        let socket: UdpSocket = UdpSocket::bind(bind).expect("Could not bind to Socket.");
        socket.set_nonblocking(true).unwrap();
        Listener {
            socket,
            callback
        }
    }

    /// non-blocking
    pub fn listen_once(&self) {
        let mut buf: [u8; 10] = [0; 10];
        if let Ok((_n_bytes, src)) = self.socket.peek_from(&mut buf) {
//            self.socket.connect(src).unwrap();
//            let conn = Connection {
//                send_jobs: Vec::new(),
//                recv_jobs: Vec::new(),
//                accept_callback: Box::new(|a| {}),
//                timeout_callback: Box::new(|| {}),
//                socket: self.socket.try_clone().unwrap(),
//                dest: src
//            };
            let connection_listener = *self.callback;
            connection_listener(src);
            todo!();
        }
    }
}
