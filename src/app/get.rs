use crate::app::frame::*;
use crate::common::*;
use crate::options::Options;
use crate::transport::client;
use crate::transport::connection::*;
use crate::transport::jobs::*;
use crate::transport::frame::*;

use log::info;
use std::net::SocketAddr;
use std::cell::RefCell;
use std::rc::Rc;
use itertools::Chunk;
use num::{FromPrimitive, ToPrimitive};

#[derive(PartialEq, Eq)]
enum State {
    Startup,
    Connected,
    TransferInProgress,
    Finished,
}

struct StateMachine {
    state: State,
    next_object_id: ObjectId,
    expected_files: Vec<String>
}

impl StateMachine {
    fn new() -> Self {
        StateMachine {
            state: State::Startup,
            next_object_id: 0,
            expected_files: vec![]
        }
    }

    fn get_next_object_id(&mut self) -> ObjectId {
        self.next_object_id += 1;
        self.next_object_id
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

    fn is_finished(&self) -> bool {
        self.state == State::Finished
    }

    fn file_request_job(&mut self, files: Vec<String>) -> ObjectSendJob {
        let files_len = files.len();
        ObjectSendJob::new(
            Object {
                object_type: AppObjectType::FileRequest.to_u8().unwrap(),
                object_id: self.get_next_object_id(),
                fields: vec![ObjectFieldDescription{
                    field_type: AppObjectFieldType::FileRequestSend.to_u8().unwrap(),
                    length: 1  // in nr. of chunks
                }],
                transmission_finished_callback: Box::new(move ||{
                    log::info!("Transmitted request for {} files.", files_len);
                }),
            },
            Box::new(move |chunk_id: ChunkId|{
                let result = FileRequest {
                    file_paths: files.clone()
                };
                let mut cursor = Cursor::new(Vec::new());
                result.write(&mut cursor);
                (cursor.into_inner(), 1)
            })
        )
    }
}

/// Run client for file retrieval.
pub fn get(opt: Options, socket_addr: SocketAddr, files: Vec<&str>) -> std::result::Result<(), ()>
{
    //////////////////////////////
    // Announce client startup.
    let mut s = format!(
        "File client startet with {} for socket address {} and file(s) '",
        opt, socket_addr
    );
    for f in &files {
        s = format!("{} {}", s, f);
    }
    info!("{} '", s);

    //////////////////////////////
    // Create shared state machine.
    let state_machine = Rc::new(RefCell::new(StateMachine::new()));
    let state_machine_for_timeout_handler = Rc::clone(&state_machine);

    //////////////////////////////
    // Create event handlers.
    let incoming_object_handler = Box::new(move |recv_job| {});
    let timeout_handler = Box::new(move || {
        state_machine_for_timeout_handler.borrow_mut().finished();
    });

    //////////////////////////////
    // Create connection.
    let mut connection = client::connect(
        socket_addr,
        incoming_object_handler,
        timeout_handler,
    );
    state_machine.borrow_mut().connected();

    //////////////////////////////
    // Request files.
    let mut files_copy = Vec::new();
    for path in files {
        files_copy.push(String::from(path));
    }
    connection.send_jobs.push(state_machine.borrow_mut().file_request_job(files_copy));

    //////////////////////////////
    // Wait until reception is done.
    while !state_machine.borrow().is_finished() {
        connection.receive_and_send();
    }

    Ok(())
}
