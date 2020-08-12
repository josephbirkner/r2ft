use crate::app::frame::*;
use crate::common::*;
use crate::options::Options;
use crate::transport::jobs::*;
use crate::transport::frame::*;
use std::path::Path;
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
use std::cmp::min;

const DEFAULT_CHUNK_SIZE:u64 = 512;

//////////////////////////////
// FileSendState

pub struct FileSendState {
    pub device: fs::File,
    pub num_content_chunks: u64,
    pub path: String
}

//////////////////////////////
// FileRecvState

pub struct FileRecvState {
    pub name: String,
    pub size: u64,
    pub device: Option<fs::File>,
    pub missing_chunks: HashSet<ChunkId>,
    pub recv_until: ChunkId
}

impl FileRecvState {
    pub fn notify_metadata(&mut self, metadata: &FileMetadata) {
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

    pub fn notify_content(&mut self, content: &FileContent, mut chunk_id: ChunkId, fields: &HashMap<ObjectFieldType, ChunkId>) {
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

    pub fn done(&self) -> bool {
        self.missing_chunks.is_empty()
    }
}

//////////////////////////////
// ObjectRecvState

pub enum ObjectRecvState {
    File(FileRecvState),
    Empty
}

impl ObjectRecvState {
    fn new(obj_type: ObjectType) -> Self {
        match FromPrimitive::from_u8(obj_type) {
            Some(AppObjectType::FileResponse) => ObjectRecvState::File(FileRecvState {
                name: String::from("unknown"),
                size: 0,
                device: None,
                missing_chunks: HashSet::new(),
                recv_until: -1
            }),
            _ => ObjectRecvState::Empty
        }
    }
}

//////////////////////////////
// StateMachine

pub struct StateMachine {
    state: State,
    next_object_id: ObjectId,
    expected_files: Vec<String>,
    recv_state: HashMap<(ObjectType, ObjectId), RefCell<ObjectRecvState>>,
    send_job_outbox: Vec<ObjectSendJob>
}

#[derive(PartialEq, Eq)]
pub enum State {
    Startup,
    Connected,
    Finished,
}

impl StateMachine
{
    pub fn new() -> Self {
        StateMachine {
            state: State::Startup,
            next_object_id: 0,
            expected_files: vec![],
            recv_state: HashMap::new(),
            send_job_outbox: vec![]
        }
    }

    pub fn get_next_object_id(&mut self) -> ObjectId {
        self.next_object_id += 1;
        self.next_object_id
    }

    pub fn connected(&mut self) {
        self.state = State::Connected;
    }

    pub fn finished(&mut self) {
        self.state = State::Finished;
    }

    pub fn is_finished(&self) -> bool {
        self.state == State::Finished
    }

    pub fn all_files_received(&self) -> bool {
        let mut num_received_files = 0;
        for transfer_state in &self.recv_state {
            let state = transfer_state.1.borrow();
            match state.deref() {
                ObjectRecvState::File(f) => {
                    if f.done() {
                        num_received_files += 1;
                    }
                },
                _ => {}
            }
        }
        num_received_files == self.expected_files.len()
    }

    pub fn push_file_request_job(&mut self, files: Vec<String>) -> ObjectSendJob {
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

    pub fn push_file_send_job(&mut self, file_path: String)
    {
        let mut file = match fs::File::open(file_path.clone()) {
            Ok(file_obj) => file_obj,
            Err(e) => {
                log::error!("Failed to open {}", file_path);
                todo!("push_error_send_job()");
                return
            }
        };
        let meta = match file.metadata() {
            Ok(meta) => meta,
            Err(e) => {
                log::error!("Failed to get metadata for {}", file_path);
                return;
            }
        };
        let mut send_state = FileSendState {
            device: file,
            num_content_chunks: (meta.len() + DEFAULT_CHUNK_SIZE - 1)/DEFAULT_CHUNK_SIZE,
            path: file_path.clone()
        };
        let new_send_job = ObjectSendJob::new(
            Object {
                object_type: AppObjectType::FileResponse.to_u8().unwrap(),
                object_id: self.get_next_object_id(),
                fields: vec![
                    ObjectFieldDescription{
                        field_type: AppObjectFieldType::FileResponseMetadata.to_u8().unwrap(),
                        length: 1  // in nr. of chunks
                    },
                    ObjectFieldDescription{
                        field_type: AppObjectFieldType::FileResponseContent.to_u8().unwrap(),
                        length: send_state.num_content_chunks as i64  // in nr. of chunks
                    },
                ],
                transmission_finished_callback: {
                    let file_path = file_path.clone();
                    Box::new(move ||{
                        log::info!("File {} fully transmitted.", file_path);
                    }
                )},
            },
            Box::new(move |chunk_id: ChunkId|{
                let path = Path::new(&send_state.path);
                let tlv_to_send = match chunk_id {
                    0 => AppTlv::FileMetadata(FileMetadata{
                        metadata_entries: vec![
                            MetadataEntry {
                                code: MetadataEntryType::FileSize,
                                content: {
                                    log::info!(" Sending metadata for {}", send_state.path);
                                    let mut cursor = Cursor::new(Vec::new());
                                    cursor.write_u64::<NetworkEndian>(meta.len()).expect("Size write failed.");
                                    cursor.into_inner()
                                }
                            },
                            MetadataEntry {
                                code: MetadataEntryType::FileName,
                                content: {
                                    let mut cursor = Cursor::new(Vec::new());
                                    cursor.write(path.file_name().unwrap().to_str().unwrap().as_bytes());
                                    cursor.into_inner()
                                }
                            }
                        ]
                    }),
                    _ => AppTlv::FileContent(FileContent{
                        content: {
                            log::info!(" Sending chunk {}/{} for {}",
                                chunk_id + 1,
                                send_state.num_content_chunks,
                                send_state.path);
                            let content_chunk_idx = chunk_id - 1; // 1 metadata chunk
                            let mut result = Vec::new();
                            let start_pos = content_chunk_idx * DEFAULT_CHUNK_SIZE as i64;
                            send_state.device.seek(SeekFrom::Start(start_pos as u64));
                            let end_pos = min(
                                start_pos + DEFAULT_CHUNK_SIZE as i64,
                                meta.len() as i64);
                            for i in start_pos..end_pos {
                                result.push(send_state.device.read_u8().unwrap());
                            }
                            result
                        }
                    })
                };
                let mut cursor = Cursor::new(Vec::new());
                tlv_to_send.write(&mut cursor);
                (cursor.into_inner(), 1)
            })
        );
        self.send_job_outbox.push(new_send_job);
    }

    pub fn has_recv_job(&self, recv_job: &ObjectReceiveJob) -> bool {
        self.recv_state.contains_key(
            &(recv_job.object.object_type, recv_job.object.object_id))
    }

    pub fn push_recv_job(state_machine: &Rc<RefCell<StateMachine>>, recv_job: &mut ObjectReceiveJob)
    {
        let object_info = (recv_job.object.object_type, recv_job.object.object_id);
        let mut field_length = HashMap::new();
        for field in &recv_job.object.fields {
            field_length.insert(field.field_type, field.length);
        }
        state_machine.borrow_mut().recv_state.insert(
            object_info.clone(),
            RefCell::new(ObjectRecvState::new(recv_job.object.object_type))
        );

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
                        (AppTlv::FileMetadata(metadata_tlv), ObjectRecvState::File(f)) => {
                            f.notify_metadata(metadata_tlv);
                        },
                        (AppTlv::FileContent(content_tlv), ObjectRecvState::File(f)) => {
                            f.notify_content(content_tlv, chunk_id, &field_length);
                        },
                        (AppTlv::ApplicationError(err_tlv), ObjectRecvState::Empty) => {
                            log::error!(" Received server error (code {})", err_tlv.error_code.to_u8().unwrap());
                            state_machine_ref.borrow_mut().finished();
                        },
                        (AppTlv::FileRequest(request_tlv), ObjectRecvState::Empty) => {
                            for file in &request_tlv.file_paths {
                                state_machine_ref.borrow_mut().push_file_send_job(file.clone());
                            }
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

    pub fn pop_new_send_job(&mut self) -> Option<ObjectSendJob> {
        self.send_job_outbox.pop()
    }
}