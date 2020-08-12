pub mod client;
pub mod connection;
pub mod frame;
pub mod jobs;
pub mod server;

mod common;

#[cfg(test)]
mod test;

/// Maximum chunksize in bytes.
pub const CHUNKSIZE: usize = 512;
