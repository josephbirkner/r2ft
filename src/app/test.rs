use super::get::get;
use super::server;
use crate::options::*;

use std::fs;
use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::thread;

#[test]
fn test_basic_file_transfer() {
    let srv_opts = Options {
        port: 38134,
        transition_probabilities: (1.0, 0.0),
    };
    let cli_opts = Options {
        port: 38135,
        transition_probabilities: (1.0, 0.0),
    };
    let srv_addr = Ipv4Addr::new(0, 0, 0, 0);
    let cli_srv_addr = SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(127, 0, 0, 1),
        srv_opts.port,
    ));

    let srv = thread::spawn(move || {
        server::run(srv_opts, srv_addr);
    });

    if fs::metadata("test.txt").is_ok() {
        fs::remove_file("test.txt");
    }
    get(cli_opts, cli_srv_addr, vec!["testdata/test.txt"]);
    {
        let mut file = fs::File::open("test.txt").unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content);
        assert_eq!(content, "Hello General Kenobi\n");
    }
    fs::remove_file("test.txt");
}
