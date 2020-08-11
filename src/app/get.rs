use crate::app::frame::*;
use crate::options::Options;
use crate::transport::client;
use crate::transport::connection::*;
use crate::transport::jobs::*;
use log::info;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(PartialEq, Eq)]
enum State {
    Startup,
    Connected,
    TransferInProgress,
    Finished,
}

struct StateMachine {
    state: State,
}

impl StateMachine {
    fn new() -> Self {
        StateMachine {
            state: State::Startup,
        }
    }

    fn connected(&mut self) {
        self.state = State::Connected;
    }

    fn transfer_in_progress(&mut self) {
        self.state = State::TransferInProgress;
    }

    fn finished(&mut self) {
        self.state = State::Finished;
    }

    fn is_finished(&mut self) -> bool {
        self.state == State::Finished
    }
}

/// Run client for file retrieval.
pub fn get(opt: Options, socket_addr: SocketAddr, files: Vec<&str>) -> std::result::Result<(), ()> {
    // Announce client startup
    let mut s = format!(
        "File client startet with {} for socket address {} and file(s) '",
        opt, socket_addr
    );

    for f in files {
        s = format!("{} {}", s, f);
    }

    info!("{} '", s);

    let state_machine = Arc::new(Mutex::new(StateMachine::new()));

    let mut next_object_id = 1;

    let incoming_object_handler = |recv_job| {};

    let state_machine_for_timeout_handler = Arc::clone(&state_machine);
    let timeout_handler = move || {
        state_machine_for_timeout_handler.lock().unwrap().finished();
    };

    let connection = client::connect(
        socket_addr,
        Box::new(incoming_object_handler),
        Box::new(timeout_handler),
    );

    state_machine.lock().unwrap().connected();

    while !state_machine.lock().unwrap().is_finished() {}

    unimplemented!();
    Ok(())
}
