use std::net::SocketAddr;
use crate::transport::connection::*;

/// Called by an application to create a `Connection`. Will bind
/// to `0.0.0.0:random` as src addr.
/// Non-blocking as it only creates state. The `Connection` will then
/// be established with handshake and everything while being granted
/// cpu_time by `Connection.grant_cpu()`.
pub fn connect(dest: SocketAddr, accept_callback: ObjectListener, timeout_callback: TimeoutListener) -> Connection {
    unimplemented!();
}
