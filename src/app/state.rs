use crate::app::frame::*;
use crate::common::*;
use crate::transport::frame::*;
use crate::transport::jobs::*;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use log::*;
use num::{FromPrimitive, ToPrimitive};
use sha3::{Digest, Sha3_512};
use std::cell::RefCell;
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::rc::Rc;
use std::cmp::PartialEq;

const DEFAULT_CHUNK_SIZE: u64 = 512;

//////////////////////////////
// FileSendState

pub struct FileSendState {
    pub device: fs::File,
    pub num_content_chunks: u64,
    pub path: String,
}

//////////////////////////////
// FileRecvState

pub struct FileRecvState {
    pub name: String,
    pub size: u64,
    pub sha3: Vec<u8>,
    pub device: Option<fs::File>,
    pub missing_chunks: HashSet<ChunkId>,
    pub num_chunks: u64,
    pub recv_until: ChunkId,
    pub header_received: bool
}

impl FileRecvState {
    pub fn notify_metadata(&mut self, metadata: &FileMetadata) {
        for entry in &metadata.metadata_entries {
            match entry.code {
                MetadataEntryType::FileName => {
                    if !self.name.is_empty() {
                        log::error!("Got file name metadata twice!");
                        continue;
                    }
                    self.name = match String::from_utf8(entry.content.clone()) {
                        Ok(name) => name,
                        Err(_) => {
                            log::error!("Failed to parse name!");
                            continue;
                        }
                    };
                    log::info!(" Got a file name: {}", self.name);
                    let file = fs::File::create(self.name.to_string()).unwrap();
                    if self.size > 0 {
                        if file.set_len(self.size).is_err() {
                            todo!("Implement error handling.");
                        }
                    }
                    self.device = Option::Some(file);
                }
                MetadataEntryType::FileSize => {
                    if self.size > 0 {
                        log::error!("Got nonzero size metadata twice!");
                        continue;
                    }
                    let mut cursor = Cursor::new(entry.content.clone());
                    self.size = match cursor.read_u64::<NetworkEndian>() {
                        Ok(size) => size,
                        Err(_) => {
                            log::error!("Failed to parse size!");
                            continue;
                        }
                    };
                    log::info!(" Got a file size: {}", self.size);
                    self.num_chunks = (self.size + DEFAULT_CHUNK_SIZE - 1) / DEFAULT_CHUNK_SIZE;
                    for i in 0..self.num_chunks {
                        self.missing_chunks.insert(i as ChunkId);
                    }
                    log::info!("  Expecting {} chunks.", self.num_chunks);
                    match &mut self.device {
                        Some(file) => file.set_len(self.size).expect("Set length failed."),
                        _ => {}
                    }
                }
                MetadataEntryType::SHA3 => {
                    if !self.sha3.is_empty() {
                        log::warn!("Got SHA3_512 metadata twice!");
                        continue;
                    }
                    let sha3 = entry.content.clone();
                    if sha3.len() != 64 {
                        log::warn!("Received SHA3 is not of length 512 bit!");
                        continue;
                    }
                    self.sha3 = sha3;
                    log::info!("Got a sha3_512 hash: {:?}", self.sha3);
                }
                _ => {}
            }
        }
        log::info!(" Header received.");
        self.header_received = true;
    }

    pub fn notify_content(
        &mut self,
        content: &FileContent,
        mut chunk_id: ChunkId,
        fields: &HashMap<ObjectFieldType, ChunkId>,
    ) {
        if self.device.is_none() {
            log::warn!("Ignoring chunk received before file was initialised.");
            return;
        }
        if self.done() {
            log::warn!("Ignoring unexpected chunk, I am done or I haven't started.");
            return;
        }
        let num_metadata_chunks =
            match fields.get(&AppObjectFieldType::FileResponseMetadata.to_u8().unwrap()) {
                Some(val) => *val,
                _ => {
                    log::warn!("The metadata field does not exist??");
                    0
                }
            };
        chunk_id -= num_metadata_chunks;
        if !self.missing_chunks.contains(&chunk_id) {
            log::warn!("Ignoring unexpected chunk with ID {}", chunk_id);
            return;
        }
        self.missing_chunks.remove(&chunk_id);
        log::info!(" Writing chunk {}/{} to {}.", chunk_id+1, self.num_chunks, self.name);
        match &mut self.device {
            Some(file) => {
                if file
                    .seek(SeekFrom::Start(chunk_id as u64 * DEFAULT_CHUNK_SIZE))
                    .is_err()
                {
                    todo!("Implement error handling.");
                }
                if file.write(content.content.as_ref()).is_err() {
                    todo!("Implement error handling.");
                }

                if chunk_id as u64+1 == self.num_chunks {

                    let mut buffer = Vec::new();
                    // Must reopen file because it was closed by last 
                    match fs::File::open(&self.name).unwrap().read_to_end(&mut buffer) {
                        Err(e) => {
                            error!("Could not produce hash for file.");
                            todo!("Implement error handling.");
                        }
                        Ok(_) => {
                            let mut hasher = Sha3_512::new();
                            hasher.update(buffer);

                            let result = hasher.finalize();

                            let mut cursor = Cursor::new(Vec::new());
                            if cursor.write(&result[..]).is_err() {
                                todo!("Implement error handling");
                            }
                            if !self.sha3.eq(&cursor.into_inner()) {
                                warn!("Received file has invalid hash.");
                                return;
                            } else {
                                trace!("Hashes matched!");
                            }
                        }
                    }
                }
            }
            _ => {
                log::error!("The file handle is gone.");
                return;
            }
        }
    }

    pub fn done(&self) -> bool {
        self.header_received && self.missing_chunks.is_empty()
    }
}

//////////////////////////////
// ObjectRecvState

pub enum ObjectRecvState {
    File(FileRecvState),
    Empty,
}

impl ObjectRecvState {
    fn new(obj_type: ObjectType) -> Self {
        match FromPrimitive::from_u8(obj_type) {
            Some(AppObjectType::FileResponse) => ObjectRecvState::File(FileRecvState {
                name: String::from(""),
                size: 0,
                sha3: vec![],
                device: None,
                missing_chunks: HashSet::new(),
                recv_until: -1,
                num_chunks: 0,
                header_received: false
            }),
            _ => ObjectRecvState::Empty,
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
    send_job_outbox: Vec<ObjectSendJob>,
}

#[derive(PartialEq, Eq)]
pub enum State {
    Startup,
    Connected,
    Finished,
}

impl StateMachine {
    pub fn new() -> Self {
        StateMachine {
            state: State::Startup,
            next_object_id: 0,
            expected_files: vec![],
            recv_state: HashMap::new(),
            send_job_outbox: vec![],
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
                }
                _ => {}
            }
        }
        num_received_files == self.expected_files.len()
    }

    pub fn push_file_request_job(&mut self, files: Vec<String>) -> ObjectSendJob {
        let files_len = files.len();
        self.expected_files = files.clone();
        ObjectSendJob::new(
            Object {
                object_type: AppObjectType::FileRequest.to_u8().unwrap(),
                object_id: self.get_next_object_id(),
                fields: vec![ObjectFieldDescription {
                    field_type: AppObjectFieldType::FileRequestSend.to_u8().unwrap(),
                    length: 1, // in nr. of chunks
                }],
                transmission_finished_callback: Box::new(move || {
                    log::info!("Transmitted request for {} files.", files_len);
                }),
            },
            Box::new(move |chunk_id: ChunkId| {
                let result = FileRequest {
                    file_paths: files.clone(),
                };
                let mut cursor = Cursor::new(Vec::new());
                result.write(&mut cursor);
                (cursor.into_inner(), 1)
            }),
        )
    }

    pub fn push_file_send_job(&mut self, file_path: String) {
        let file = match fs::File::open(file_path.clone()) {
            Ok(file_obj) => file_obj,
            Err(_) => {
                log::error!("Failed to open {}", file_path);
                todo!("push_error_send_job()");
                /*self.push_error_send_job(ApplicationError {
                    error_code: AppErrorCode::FileAbort,
                    error_data: AppErrorData::Paths(vec![file_path.clone()]),
                });
                return;*/
            }
        };
        let meta = match file.metadata() {
            Ok(meta) => meta,
            Err(_) => {
                log::error!("Failed to get metadata for {}", file_path);
                return;
            }
        };
        let mut send_state = FileSendState {
            device: file,
            num_content_chunks: (meta.len() + DEFAULT_CHUNK_SIZE - 1) / DEFAULT_CHUNK_SIZE,
            path: file_path.clone(),
        };
        let new_send_job = ObjectSendJob::new(
            Object {
                object_type: AppObjectType::FileResponse.to_u8().unwrap(),
                object_id: self.get_next_object_id(),
                fields: vec![
                    ObjectFieldDescription {
                        field_type: AppObjectFieldType::FileResponseMetadata.to_u8().unwrap(),
                        length: 1, // in nr. of chunks
                    },
                    ObjectFieldDescription {
                        field_type: AppObjectFieldType::FileResponseContent.to_u8().unwrap(),
                        length: send_state.num_content_chunks as i64, // in nr. of chunks
                    },
                ],
                transmission_finished_callback: {
                    let file_path = file_path.clone();
                    Box::new(move || {
                        log::info!("File {} fully transmitted.", file_path);
                    })
                },
            },
            Box::new(move |chunk_id: ChunkId| {
                let path = Path::new(&send_state.path);
                let tlv_to_send = match chunk_id {
                    0 => AppTlv::FileMetadata(FileMetadata {
                        metadata_entries: vec![
                            MetadataEntry {
                                code: MetadataEntryType::FileSize,
                                content: {
                                    log::info!(" Sending metadata for {}", send_state.path);
                                    let mut cursor = Cursor::new(Vec::new());
                                    if cursor.write_u64::<NetworkEndian>(meta.len()).is_err() {
                                        todo!("Implement error handling");
                                    }
                                    cursor.into_inner()
                                },
                            },
                            MetadataEntry {
                                code: MetadataEntryType::FileName,
                                content: {
                                    let mut cursor = Cursor::new(Vec::new());
                                    if cursor
                                        .write(
                                            path.file_name().unwrap().to_str().unwrap().as_bytes(),
                                        )
                                        .is_err()
                                    {
                                        todo!("Implement error handling");
                                    }
                                    cursor.into_inner()
                                },
                            },
                            MetadataEntry {
                                code: MetadataEntryType::SHA3,
                                content: {
                                    let mut buffer = Vec::new();

                                    match send_state.device.read_to_end(&mut buffer) {
                                        Err(e) => {
                                            error!("Could not produce hash for file.");
                                            todo!("Implement error handling.");
                                        }
                                        Ok(_) => {
                                            let mut hasher = Sha3_512::new();
                                            hasher.update(buffer);

                                            let result = hasher.finalize();

                                            let mut cursor = Cursor::new(Vec::new());
                                            if cursor.write(&result[..]).is_err() {
                                                todo!("Implement error handling");
                                            }
                                            cursor.into_inner()
                                        }
                                    }
                                },
                            },
                        ],
                    }),
                    _ => AppTlv::FileContent(FileContent {
                        content: {
                            let content_chunk_idx = chunk_id - 1; // 1 metadata chunk
                            log::info!(
                                " Sending chunk {}/{} for {}",
                                content_chunk_idx + 1,
                                send_state.num_content_chunks,
                                send_state.path
                            );
                            let mut result = Vec::new();
                            let start_pos = content_chunk_idx * DEFAULT_CHUNK_SIZE as i64;
                            if send_state
                                .device
                                .seek(SeekFrom::Start(start_pos as u64))
                                .is_err()
                            {
                                todo!("Implement error handling.");
                            }
                            let end_pos =
                                min(start_pos + DEFAULT_CHUNK_SIZE as i64, meta.len() as i64);
                            for _ in start_pos..end_pos {
                                result.push(send_state.device.read_u8().unwrap());
                            }
                            result
                        },
                    }),
                };
                let mut cursor = Cursor::new(Vec::new());
                tlv_to_send.write(&mut cursor);
                (cursor.into_inner(), 1)
            }),
        );
        self.send_job_outbox.push(new_send_job);
    }

    pub fn push_error_send_job(&mut self, app_err: ApplicationError) {
        // The given application error will be sent via a single TLV (thus will not work for large payloads)
        // extending to large lists of file paths probably won't make much sense.
        let new_send_job = ObjectSendJob::new(
            Object {
                object_type: AppObjectType::ErrorReport.to_u8().unwrap(),
                object_id: self.get_next_object_id(),
                fields: vec![ObjectFieldDescription {
                    field_type: AppObjectFieldType::ErrorReportContent.to_u8().unwrap(),
                    length: 1, // in nr. of chunks
                }],
                transmission_finished_callback: Box::new(move || {
                    log::info!("Error fully transmitted.");
                }),
            },
            Box::new(move |chunk_id: ChunkId| {
                let mut cursor = Cursor::new(Vec::new());
                app_err.write(&mut cursor);
                (cursor.into_inner(), 1)
            }),
        );
        self.send_job_outbox.push(new_send_job);
    }

    pub fn has_recv_job(&self, recv_job: &ObjectReceiveJob) -> bool {
        self.recv_state
            .contains_key(&(recv_job.object.object_type, recv_job.object.object_id))
    }

    pub fn push_recv_job(
        state_machine: &Rc<RefCell<StateMachine>>,
        recv_job: &mut ObjectReceiveJob,
    ) {
        let object_info = (recv_job.object.object_type, recv_job.object.object_id);
        let mut field_length = HashMap::new();
        for field in &recv_job.object.fields {
            field_length.insert(field.field_type, field.length);
        }
        state_machine.borrow_mut().recv_state.insert(
            object_info.clone(),
            RefCell::new(ObjectRecvState::new(recv_job.object.object_type)),
        );

        let state_machine_ref = Rc::clone(state_machine);
        recv_job.chunk_received_callback =
            Box::new(move |data: Vec<u8>, chunk_id: ChunkId, num_tlv: u8| {
                let mut state_machine = state_machine_ref.borrow_mut();
                let mut cursor = Cursor::new(data);
                log::info!(
                    "Received chunk #{} for object #{}, {} tlvs.",
                    chunk_id,
                    object_info.1,
                    num_tlv
                );
                let mut tlv_idx = 0;
                while tlv_idx < num_tlv {
                    log::info!("Parsing TLV #{} ...", tlv_idx);
                    let tlv = match parse(&mut cursor) {
                        AppTlvParseResult::Ok(tlv) => tlv,
                        AppTlvParseResult::Err(e) => {
                            log::error!(" Error: {}", e);
                            return;
                        }
                    };
                    let obj_state = state_machine.recv_state.get(&object_info).unwrap();
                    let mut finished = false;
                    let mut new_file_send_jobs = vec![];
                    match (&tlv, obj_state.borrow_mut().deref_mut()) {
                        (AppTlv::FileMetadata(metadata_tlv), ObjectRecvState::File(f)) => {
                            f.notify_metadata(metadata_tlv);
                        }
                        (AppTlv::FileContent(content_tlv), ObjectRecvState::File(f)) => {
                            f.notify_content(content_tlv, chunk_id, &field_length);
                        }
                        (AppTlv::ApplicationError(err_tlv), ObjectRecvState::Empty) => {
                            log::error!(
                                " Received server error (code {})",
                                err_tlv.error_code.to_u8().unwrap()
                            );
                            finished = true;
                        }
                        (AppTlv::FileRequest(request_tlv), ObjectRecvState::Empty) => {
                            new_file_send_jobs = request_tlv.file_paths.clone();
                        }
                        _ => {
                            log::error!(" Encountered unexpected TLV type.");
                            finished = true;
                        }
                    }
                    tlv_idx += 1;
                    if finished {
                        state_machine.finished();
                    }
                    for file in &new_file_send_jobs {
                        state_machine.push_file_send_job(file.clone());
                    }
                }
            });
    }

    pub fn pop_new_send_job(&mut self) -> Option<ObjectSendJob> {
        self.send_job_outbox.pop()
    }
}
