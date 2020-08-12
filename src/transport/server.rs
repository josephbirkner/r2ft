use std::net::{SocketAddr, UdpSocket};
use crate::common::udp::Socket;
use crate::transport::connection::*;

/// Is called by the transport layer to inform the application
/// about new connections. The server-application then returns its
/// ObjectListener for that connection.
/// TODO: fn(incoming: SocketAddr) and then app shall call connect on that addr
pub type ConnectionListener = fn (incoming: Connection) -> ObjectListener;

pub struct Listener {
    socket: UdpSocket,
    callback: ConnectionListener,
}

impl Listener {
    /// Used by servers to listen for incoming connections.
    /// Install a ConnectionListener to be called for each new
    /// incoming connection. Will listen at `bind`.
    /// Non-blocking.
    pub fn listen(bind: SocketAddr, callback: ConnectionListener) -> Listener {
        let socket: UdpSocket = UdpSocket::bind(bind).expect("Could not bind to Socket.");
        socket.set_nonblocking(true);
        Listener {
            socket,
            callback
        }
    }

    pub fn grant_cpu(&self) {
        let mut buf: [u8; 10] = [0; 10];
        if let Ok((n_bytes, src)) = self.socket.peek_from(&mut buf) {
            self.socket.connect(src);
            let conn = Connection {
                send_jobs: Vec::new(),
                recv_jobs: Vec::new(),
                accept_callback: |a| {},
                timeout_callback: || {},
                socket: self.socket,
                dest: src
            };
            self.callback(conn);
        }
    }
}
