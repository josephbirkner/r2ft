use crate::options::Options;
use crate::transport::connection::*;
use log::info;
use std::net::SocketAddr;

/// Run client for file list retrieval.
pub fn ls(opt: Options, socket_addr: SocketAddr, directory: &str) -> std::result::Result<(), ()> {
    info!(
        "File list client startet with {} for socket address {} and directory {}",
        opt, socket_addr, directory
    );
    unimplemented!();
    Ok(())
}
