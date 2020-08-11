//! # RFT
//! Rust implementation of the SOFT (Simple One-Directional File Transfer) protocol.

extern crate num;
#[macro_use]
extern crate num_derive;

#[macro_use]
pub mod common;
pub mod transport;
pub mod app;
pub mod options;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
