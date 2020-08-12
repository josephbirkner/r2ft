use std::net::{SocketAddr, UdpSocket};
use crate::common::udp::{Socket};
use crate::transport::connection::*;

use rand::{thread_rng, Rng};

/// Called by an application to create a `Connection`. Will bind
/// to `0.0.0.0:random` as src addr.
/// Non-blocking as it only creates state. The `Connection` will then
/// be established with handshake and everything while being granted
/// cpu_time by `Connection.grant_cpu()`.
pub fn connect(dest: SocketAddr, accept_callback: Box<ObjectListener>, timeout_callback: Box<TimeoutListener>) -> Connection {
    // bind to a random local port from ephemeral port range
    let bind: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let port: u16 = thread_rng().gen_range(49152, 65535);
    bind.set_port(port);

    let socket: UdpSocket = UdpSocket::bind(bind).expect("Could not bind to Socket.");
    socket.set_nonblocking(true);
    socket.connect(dest);

    let conn = Connection {
        send_jobs: Vec::new(),
        recv_jobs: Vec::new(),
        accept_callback,
        timeout_callback,
        socket,
        dest
    };

    return conn;
}
