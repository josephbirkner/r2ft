use crate::app::frame::*;
use crate::common::*;
use crate::options::Options;
use crate::transport::client;
use crate::transport::jobs::*;
use crate::transport::frame::*;
use byteorder::{NetworkEndian, WriteBytesExt, ReadBytesExt};

use std::fs;
use std::io::{Seek, SeekFrom, Write, Read};
use std::collections::{HashMap, HashSet};
use log::info;
use log::error;
use std::net::SocketAddr;
use std::cell::RefCell;
use std::rc::Rc;
use itertools::Chunk;
use num::{FromPrimitive, ToPrimitive};
use std::process::exit;
use std::ops::{DerefMut, Deref};

const DEFAULT_CHUNK_SIZE:u64 = 512;

#[derive(PartialEq, Eq)]
enum State {
    Startup,
    Connected,
    Finished,
}

struct FileReceiveState {
    name: String,
    size: u64,
    device: Option<fs::File>,
    missing_chunks: HashSet<ChunkId>,
    all_received_until: ChunkId
}

impl FileReceiveState {
    fn notify_metadata(&mut self, metadata: &FileMetadata) {
        for entry in &metadata.metadata_entries {
            match entry.code {
                MetadataEntryType::FileName => {
                    if !self.name.is_empty() {
                        log::error!("Got file name metadata twice!");
                        continue
                    }
                    self.name = match String::from_utf8(entry.content.clone()) {
                        Ok(name) => name,
                        Err(e) => {log::error!("Failed to parse name!"); continue}
                    };
                    log::info!(" Got a file name: {}", self.name);
                    let mut file = fs::File::create(self.name.to_string()).unwrap();
                    if self.size > 0 {
                        file.set_len(self.size);
                    }
                    self.device = Option::Some(file);
                },
                MetadataEntryType::FileSize => {
                    if self.size > 0 {
                        log::error!("Got nonzero size metadata twice!");
                        continue
                    }
                    let mut cursor = Cursor::new(entry.content.clone());
                    self.size = match cursor.read_u64::<NetworkEndian>() {
                        Ok(size) => size,
                        Err(e) => {log::error!("Failed to parse size!"); continue}
                    };
                    log::info!(" Got a file size: {}", self.size);
                    let num_chunks = (self.size + DEFAULT_CHUNK_SIZE - 1) / DEFAULT_CHUNK_SIZE;
                    for i in 0..num_chunks {
                        self.missing_chunks.insert(i as ChunkId);
                    }
                    log::info!("  Expecting {} chunks.", self.missing_chunks.len());
                    match &mut self.device {
                        Some(file) => file.set_len(self.size).expect("Set length failed."),
                        _ => {}
                    }
                },
                _ => {}
            }
        }
    }

    fn notify_content(&mut self, content: &FileContent, mut chunk_id: ChunkId, fields: &HashMap<ObjectFieldType, ChunkId>) {
        if self.device.is_none() {
            log::warn!("Ignoring chunk received before file was initialised.");
            return
        }
        if self.done() {
            log::warn!("Ignoring unexpected chunk, I am done or I haven't started.");
            return
        }
        let num_metadata_chunks = match fields.get(&AppObjectFieldType::FileResponseMetadata.to_u8().unwrap()) {
            Some(val) => *val,
            _ => {log::warn!("The metadata field does not exist??"); 0}
        };
        chunk_id -= num_metadata_chunks;
        if !self.missing_chunks.contains(&chunk_id) {
            log::warn!("Ignoring unexpected chunk with ID {}", chunk_id);
            return
        }
        self.missing_chunks.remove(&chunk_id);
        log::info!(" Writing chunk {} to {}.", chunk_id, self.name);
        match &mut self.device {
            Some(file) => {
                file.seek(SeekFrom::Start(chunk_id as u64 * DEFAULT_CHUNK_SIZE));
                file.write(content.content.as_ref());
            },
            _ => {log::error!("The file handle is gone."); return}
        }
    }

    fn done(&self) -> bool {
        self.missing_chunks.is_empty()
    }
}

enum ObjectTransferState {
    File(FileReceiveState),
}

impl Default for ObjectTransferState {
    fn default() -> Self {
        ObjectTransferState::File(FileReceiveState{
            name: String::from("unknown"),
            size: 0,
            device: None,
            missing_chunks: HashSet::new(),
            all_received_until: -1
        })
    }
}

struct StateMachine {
    state: State,
    next_object_id: ObjectId,
    expected_files: Vec<String>,
    recv_state: HashMap<(ObjectType, ObjectId), RefCell<ObjectTransferState>>
}

impl StateMachine {
    fn new() -> Self {
        StateMachine {
            state: State::Startup,
            next_object_id: 0,
            expected_files: vec![],
            recv_state: HashMap::new()
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

    fn all_files_received(&self) -> bool {
        let mut num_received_files = 0;
        for transfer_state in &self.recv_state {
            let state = transfer_state.1.borrow();
            match state.deref() {
                ObjectTransferState::File(f) => {
                    if f.done() {
                        num_received_files += 1;
                    }
                }
            }
        }
        num_received_files == self.expected_files.len()
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
        self.recv_state.contains_key(
            &(recv_job.object.object_type, recv_job.object.object_id))
    }

    fn register_recv_job(state_machine: &Rc<RefCell<StateMachine>>, recv_job: &mut ObjectReceiveJob)
    {
        let object_info = (recv_job.object.object_type, recv_job.object.object_id);
        let mut field_length = HashMap::new();
        for field in &recv_job.object.fields {
            field_length.insert(field.field_type, field.length);
        }
        state_machine.borrow_mut().recv_state.insert(object_info.clone(), RefCell::new(ObjectTransferState::default()));
        let state_machine_ref = Rc::clone(state_machine);
        recv_job.chunk_received_callback = Box::new(
            move |data: Vec<u8>, chunk_id: ChunkId, num_tlv: u8|
            {
                let state_machine = state_machine_ref.borrow_mut();
                let mut cursor = Cursor::new(data);
                log::info!("Received chunk #{} for object #{}, {} tlvs.", chunk_id, object_info.1, num_tlv);
                let mut tlv_idx = 0;
                while tlv_idx < num_tlv
                {
                    log::info!("Parsing TLV #{} ...", tlv_idx);
                    let tlv = match parse(&mut cursor) {
                        AppTlvParseResult::Ok(tlv) => tlv,
                        AppTlvParseResult::Err(e) => {
                            log::error!(" Error: {}", e);
                            return
                        }
                    };
                    let obj_state = state_machine.recv_state.get(&object_info).unwrap();
                    match (&tlv, obj_state.borrow_mut().deref_mut()) {
                        (AppTlv::FileMetadata(metadata_tlv), ObjectTransferState::File(f)) => {
                            f.notify_metadata(metadata_tlv);
                        },
                        (AppTlv::FileContent(content_tlv), ObjectTransferState::File(f)) => {
                            f.notify_content(content_tlv, chunk_id, &field_length);
                        },
                        (AppTlv::ApplicationError(err_tlv), _) => {
                            log::error!(" Received server error (code {})", err_tlv.error_code.to_u8().unwrap());
                            state_machine_ref.borrow_mut().finished();
                        },
                        _ => {
                            log::error!(" Encountered unexpected TLV type.");
                            state_machine_ref.borrow_mut().finished();
                        }
                    }
                    tlv_idx += 1;
                }
            }
        );
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
    while !state_machine.borrow().is_finished()
    {
        connection.receive_and_send();

        for recv_job in &mut connection.recv_jobs {
            if !state_machine.borrow().is_recv_job_registered(recv_job) {
                StateMachine::register_recv_job(&state_machine, recv_job);
            }
        }

        if state_machine.borrow().all_files_received() {
            state_machine.borrow_mut().finished();
        }
    }

    Ok(())
}
