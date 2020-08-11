//! # RFT
//! Rust implementation of the SOFT (Simple One-Directional File Transfer) protocol.

pub mod options;

/*
 * return exit code of executable
 */
pub fn run_client(server: &str, path: &str, dest: Option<&str>) -> i32 {
    println!("download stub");
    return 0;
}

pub fn run_server(directory: &str) -> i32 {
    println!("server stub");
    return 0;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
