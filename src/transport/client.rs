use crate::transport::common::default_host_info;
use crate::transport::connection::*;
use std::net::{SocketAddr, UdpSocket};

use rand::{thread_rng, Rng};

/// Called by an application to create a `Connection`. Will bind
/// to `0.0.0.0:random` as src addr.
/// Non-blocking as it only creates state. The `Connection` will then
/// be established with handshake and everything while being granted
/// cpu_time by `Connection.grant_cpu()`.
pub fn connect(
    dest: SocketAddr,
    accept_callback: Box<ObjectListener>,
    timeout_callback: Box<TimeoutListener>,
) -> Connection {
    // bind to a random local port from ephemeral port range
    let bind: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let port: u16 = thread_rng().gen_range(49152, 65535);
    bind.set_port(port);

    let socket: UdpSocket = UdpSocket::bind(bind).expect("Could not bind to Socket.");
    socket.set_nonblocking(true).unwrap();
    socket.connect(dest).unwrap();

    let mut conn = Connection {
        send_jobs: Vec::new(),
        recv_jobs: Vec::new(),
        accept_callback,
        timeout_callback,
        socket,
        dest,
        is_server: false,
        self_info: default_host_info(),
        peer_info: None,
        session: None,
    };

    conn.send_handshake();

    return conn;
}
