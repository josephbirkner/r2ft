use log::info;
use std::net::SocketAddr;
use crate::options::Options;
use crate::transport::connection::*;

/// Run server on current working directory
pub fn run(opt: Options) -> std::result::Result<(), ()> {
    info!("Server startet with {}", opt);
    unimplemented!();
    Ok(())
}
