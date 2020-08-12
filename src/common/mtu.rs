use std::fs::{metadata, read_dir, read_to_string};
use std::option::Option;

/// 0xffff - (sizeof(IP Header) + sizeof(UDP Header)) = 65535-(20+8) = 65507
const ASSUMED_IP_UDP_HEADERS: u32 = 20 + 8;
pub const UDP_PAYLOAD_MAX: u32 = 65507;

/// Dirty hack to quickly get an MTU set by unix.
/// another approach on unix would be ioctl
/// https://serverfault.com/questions/361503/getting-interface-mtu-under-linux-with-pcap
fn unix_virtual_file() -> Option<u32> {
    if let Ok(dir) = read_dir("/sys/class/net/") {
        // we are probably on linux and will find an mtu
        for entry in dir {
            let entry = entry.unwrap();

            // loopback devices tend to have bigger mtus
            if entry.file_name() == "lo" {
                continue;
            }

            let mut content = read_to_string(entry.path().join("mtu")).unwrap();
            content.truncate(content.len() - 1); // strip newline
            return Some(content.parse::<u32>().unwrap());
        }
    }
    return None;
}

/// Best effort of figuring out the MTU of the link and return
/// the corresponding udp payload size in bytes.
pub fn udp_payload_default() -> u32 {
    let mtu: u32 = match unix_virtual_file() {
        Some(mtu) => mtu,
        None => 1500, // fallback mtu
    };
    return mtu - ASSUMED_IP_UDP_HEADERS;
}

#[test]
fn mtu() {
    println!("from_virtual_file: {}", unix_virtual_file().unwrap_or(0));
    println!("udp_payload: {}", udp_payload_default());
}
