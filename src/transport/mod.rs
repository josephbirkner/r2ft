
pub mod frame;
pub mod jobs;
pub mod connection;
pub mod server;
pub mod client;

#[cfg(test)]
mod test;

/// Maximum chunksize in bytes.
pub const CHUNKSIZE: usize = 512;

