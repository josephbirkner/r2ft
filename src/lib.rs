//! # RFT
//! Rust implementation of the SOFT (Simple One-Directional File Transfer) protocol.

extern crate num;
#[macro_use]
extern crate num_derive;

#[macro_use]
mod common;
mod transport;
mod app;
pub mod options;

use log::info;
use options::Options;
use std::net::SocketAddr;
use transport::connection::*;


/// Run server on current working directory
pub fn run_server(opt: Options) -> std::result::Result<(), ()> {
    info!("Server startet with {}", opt);
    unimplemented!();
    Ok(())
}

/// Run client for file list retrieval.
pub fn run_ls_client(
    opt: Options,
    socket_addr: SocketAddr,
    directory: &str,
) -> std::result::Result<(), ()> {
    info!(
        "File list client startet with {} for socket address {} and directory {}",
        opt, socket_addr, directory
    );
    unimplemented!();
    Ok(())
}

/// Run client for file retrieval.
pub fn run_client(
    opt: Options,
    socket_addr: SocketAddr,
    files: Vec<&str>,
) -> std::result::Result<(), ()> {
    !unimplemented!()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
