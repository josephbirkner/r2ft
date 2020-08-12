use crate::transport::frame::*;

pub const APP_VERSION: Version = 0;
pub const PROTOCOL_VERSION: Version = 2;
pub const MAX_UDP_BUFSIZE: usize = 9000;

//////////////////////
// util

pub fn get_host_os() -> HostOs {
    log::info!("Assuming HostOs Linux.");
    HostOs::Linux
}

pub fn default_host_info() -> HostInformation {
    HostInformation {
        rcv_window_size: 50,
        out_of_order_limit: 50,
        ack_freq: AckFreq::Max,
        os: get_host_os(),
        app: ApplicationId::SOFT,
        app_ver: APP_VERSION,
    }
}
