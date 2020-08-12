use log::info;
use std::net::SocketAddr;
use crate::options::Options;
use crate::transport::connection::*;
use crate::transport::client;
use crate::transport::jobs::*;
use crate::app::frame::*;

/// Run client for file retrieval.
pub fn get(
    opt: Options,
    socket_addr: SocketAddr,
    files: Vec<&str>,
) -> std::result::Result<(), ()>
{
    // Announce client startup
    info!(
        "File client startet with {} for socket address {} and file(s):",
        opt, socket_addr
    );
    for file in &files {
        info!(" {}", file);
    }
    info!("\n");

    #[derive(PartialEq,Eq)]
    enum State {
        Startup,
        Connected,
        TransferInProgress,
        Finished
    }

    let mut state = State::Startup;
    let mut next_object_id = 1;

    let incoming_object_handler = |recv_job| {

    };

    let timeout_handler = || {
        state = State::Finished;
    };

    let connection = client::connect(
        socket_addr,
        Box::new(incoming_object_handler),
        Box::new(timeout_handler));
    state = State::Connected;

    while state != State::Finished {
    }

    unimplemented!();
    Ok(())
}
