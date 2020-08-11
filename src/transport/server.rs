use std::net::SocketAddr;
use crate::transport::connection::*;

/// Is called by the transport layer to inform the application
/// about new connections. The server-application then returns its
/// ObjectListener for that connection.
pub type ConnectionListener = fn (incoming: Connection) -> ObjectListener;

/// Used by servers to listen for incoming connections.
/// Install a ConnectionListener to be called for each new
/// incoming connection. Will listen at `bind`.
/// Non-blocking.
pub fn listen(bind: SocketAddr, callback: ConnectionListener) -> () {
    todo!();
}
