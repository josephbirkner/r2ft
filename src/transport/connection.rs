use crate::transport::jobs::*;
use std::net::SocketAddr;

//////////////////////////
// Connection

/// Will be called by the transport layer to notify the application
/// about new receiving Objects.
/// The application returns None, if it is not interested in the Object.
/// Otherwise the application returns its ChunkListener for that Object.
pub type ObjectListener = fn (receiver: ObjectReceiveJob) -> ();

/// Will be called by the transport layer to inform the application
/// about a timeout of a connection.
pub type TimeoutListener = fn () -> ();

pub struct Connection {
    pub send_jobs: Vec<ObjectSendJob>,
    pub recv_jobs: Vec<ObjectReceiveJob>,
    accept_callback: ObjectListener,
    timeout_callback: TimeoutListener,
}

impl Connection{
    /// Should return within about 0.1s to allow the application to interact
    /// with the user still.
    /// Must be called by the application in its main loop.
    pub fn receive_and_send(&mut self)
    {
        todo!();
    }

    pub fn new(addr: SocketAddr, acceptor: ObjectListener, timeout_handler: TimeoutListener) -> Self {
        !unimplemented!();
    }
}
