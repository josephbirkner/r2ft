use crate::app::frame::*;
use crate::common::*;
use crate::options::Options;
use crate::transport::client;
use crate::transport::jobs::*;
use crate::transport::frame::*;

use std::collections::{HashMap, HashSet};
use log::info;
use log::error;
use std::net::SocketAddr;
use std::cell::RefCell;
use std::rc::Rc;
use itertools::Chunk;
use num::{FromPrimitive, ToPrimitive};

#[derive(PartialEq, Eq)]
enum State {
    Startup,
    Connected,
    Finished,
}

enum ObjectTransferState {
    File {

    },
}

struct StateMachine {
    state: State,
    next_object_id: ObjectId,
    expected_files: Vec<String>,
    receiving_objects: HashSet<(ObjectType, ObjectId)> // HashMap<(ObjectType, ObjectId), ObjectTransferState>
}

impl StateMachine {
    fn new() -> Self {
        StateMachine {
            state: State::Startup,
            next_object_id: 0,
            expected_files: vec![],
            receiving_objects: HashSet::new()
        }
    }

    fn get_next_object_id(&mut self) -> ObjectId {
        self.next_object_id += 1;
        self.next_object_id
    }

    fn connected(&mut self) {
        self.state = State::Connected;
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

    fn is_recv_job_registered(&self, recv_job: &ObjectReceiveJob) -> bool {
        self.receiving_objects.contains(
            &(recv_job.object.object_type, recv_job.object.object_id))
    }

    fn register_recv_job(&mut self, recv_job: &mut ObjectReceiveJob) {
        let object_info = (recv_job.object.object_type, recv_job.object.object_id);
        let mut object_fields = Vec::new();
        for field in &recv_job.object.fields {
            object_fields.push(field.clone());
        }
        self.receiving_objects.insert(object_info.clone());
        recv_job.chunk_received_callback = Box::new(move |data: Vec<u8>, chunk_id: ChunkId, num_tlv: u8| {
            log::info!("Received chunk #{} for object #{}, {} tlvs.", chunk_id, object_info.1, num_tlv);
            let mut tlv_idx = 0;
            let tlv = match parse(&mut Cursor::new(data)) {
                AppTlvParseResult::Ok(tlv) => tlv,
                AppTlvParseResult::Err(e) => {
                    log::error!(" Error: {}", e);
                    return
                }
            };
            match tlv {
                AppTlv::FileMetadata(metadata_tlv) => {

                },
                AppTlv::FileContent(content_tlv) => {

                },
                AppTlv::ApplicationError(err_tlv) => {

                },
                _ => {
                    log::error!(" Encountered unexpected TLV type.");
                }
            }
        });
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
        for recv_job in &mut connection.recv_jobs {
            if !state_machine.borrow().is_recv_job_registered(recv_job) {
                state_machine.borrow_mut().register_recv_job(recv_job);
            }
        }
    }

    Ok(())
}
