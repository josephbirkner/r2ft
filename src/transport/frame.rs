use crate::common::fnv1a32;
use crate::common::*;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use leb128;
use num::{FromPrimitive, ToPrimitive};
use std::io::{Read, Seek, SeekFrom, Write};

/////////////////////////////////
// Basic Types

pub type Version = u8;
pub type SessionId = u64;
pub type ObjectId = u64;
pub type ChunkId = i64; // Signed because header is chunk -1
pub type ObjectType = u8;
pub type ObjectFieldType = u8;

/////////////////////////////////
// MessageFrame

#[derive(Default, Debug, PartialEq)]
pub struct MessageFrame {
    pub version: Version,
    pub sid: SessionId,
    pub tlvs: Vec<Tlv>,
}

impl WireFormat for MessageFrame {
    fn write(&self, cursor: &mut Cursor) {
        let start = cursor.position();
        write_u8!(cursor, self.version);
        write_u64!(cursor, self.sid);
        write_u8!(cursor, self.tlvs.len() as u8);
        for tlv in &self.tlvs {
            tlv.write(cursor);
        }
        let end = cursor.position();
        let checksum = fnv1a32::Fnv32a::hash(cursor, start, end);
        write_u32!(cursor, checksum);
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        let start = cursor.position();
        self.version = read_u8!(cursor);
        self.sid = read_u64!(cursor);
        self.tlvs = Vec::new();
        let mut num_tlvs = read_u8!(cursor);
        while num_tlvs > 0 {
            let tlv_type = read_u8!(cursor);
            cursor
                .seek(SeekFrom::Current(-1))
                .expect("Seek back failed."); // TLV will read type again.
            let mut tlv = match FromPrimitive::from_u8(tlv_type) {
                Some(TlvType::HostInformation) => Tlv::HostInformation(HostInformation::default()),
                Some(TlvType::ObjectHeader) => Tlv::ObjectHeader(ObjectHeader::default()),
                Some(TlvType::ObjectChunk) => Tlv::ObjectChunk(ObjectChunk::default()),
                Some(TlvType::ObjectSkip) => Tlv::ObjectSkip(ObjectSkip::default()),
                Some(TlvType::ObjectAck) => Tlv::ObjectAck(ObjectAck::default()),
                Some(TlvType::ErrorMessage) => Tlv::ErrorMessage(ErrorMessage::default()),
                Some(TlvType::ObjectAckRequest) => {
                    Tlv::ObjectAckRequest(ObjectAckRequest::default())
                }
                None => {
                    return ReadResult::Err(ReadError::new(
                        format!("Unknown transport message type code {}!", tlv_type).as_str(),
                    ))
                }
            };
            tlv.read(cursor);
            self.tlvs.push(tlv);
            num_tlvs -= 1;
        }
        let end = cursor.position();
        let checksum = fnv1a32::Fnv32a::hash(cursor, start, end);
        let advertised_checksum = read_u32!(cursor);
        if checksum != advertised_checksum {
            return ReadResult::Err(ReadError::new("Checksum error!"));
        }
        ReadResult::Ok
    }
}

/////////////////////////////////
// Tlv

#[derive(Debug, PartialEq)]
pub enum Tlv {
    HostInformation(HostInformation),
    ObjectHeader(ObjectHeader),
    ObjectChunk(ObjectChunk),
    ObjectSkip(ObjectSkip),
    ObjectAck(ObjectAck),
    ErrorMessage(ErrorMessage),
    ObjectAckRequest(ObjectAckRequest),
}

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq)]
#[repr(u8)]
enum TlvType {
    HostInformation = 0x50,
    ObjectHeader = 0x51,
    ObjectChunk = 0x52,
    ObjectSkip = 0x53,
    ObjectAck = 0x30,
    ErrorMessage = 0x31,
    ObjectAckRequest = 0x32,
}

impl WireFormat for Tlv {
    fn write(&self, cursor: &mut Cursor) {
        match self {
            Tlv::HostInformation(x) => x.write(cursor),
            Tlv::ObjectHeader(x) => x.write(cursor),
            Tlv::ObjectChunk(x) => x.write(cursor),
            Tlv::ObjectSkip(x) => x.write(cursor),
            Tlv::ObjectAck(x) => x.write(cursor),
            Tlv::ErrorMessage(x) => x.write(cursor),
            Tlv::ObjectAckRequest(x) => x.write(cursor),
        }
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        return match self {
            Tlv::HostInformation(x) => x.read(cursor),
            Tlv::ObjectHeader(x) => x.read(cursor),
            Tlv::ObjectChunk(x) => x.read(cursor),
            Tlv::ObjectSkip(x) => x.read(cursor),
            Tlv::ObjectAck(x) => x.read(cursor),
            Tlv::ErrorMessage(x) => x.read(cursor),
            Tlv::ObjectAckRequest(x) => x.read(cursor),
        };
    }
}

/////////////////////////////////
// HostInformation

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq, Clone)]
#[repr(u8)]
pub enum AckFreq {
    Default = 0x0,
    Min = 0x10,
    Max = 0x11,
}

impl Default for AckFreq {
    fn default() -> Self {
        AckFreq::Default
    }
}

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq, Clone)]
#[repr(u8)]
pub enum HostOs {
    Linux = 1,
    Windows = 2,
    MacOS = 3,
    FreeBSD = 4,
    Android = 5,
    IOS = 6,
}

impl Default for HostOs {
    fn default() -> Self {
        HostOs::Linux
    }
}

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq, Clone)]
#[repr(u8)]
pub enum ApplicationId {
    SOFT = 1,
}

impl Default for ApplicationId {
    fn default() -> Self {
        ApplicationId::SOFT
    }
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct HostInformation {
    pub rcv_window_size: u64, // LEB128
    pub out_of_order_limit: u8,
    pub ack_freq: AckFreq,
    pub os: HostOs,
    pub app: ApplicationId,
    pub app_ver: Version,
}

impl WireFormat for HostInformation {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, TlvType::HostInformation, {
            write_u128!(cursor, self.rcv_window_size);
            write_u8!(cursor, self.out_of_order_limit);
            write_u8!(cursor, self.ack_freq.to_u8().unwrap());
            write_u8!(cursor, self.os.to_u8().unwrap());
            write_u8!(cursor, self.app.to_u8().unwrap());
            write_u8!(cursor, self.app_ver);
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, TlvType::HostInformation, {
            self.rcv_window_size = read_u128!(cursor);
            self.out_of_order_limit = read_u8!(cursor);
            self.ack_freq = match FromPrimitive::from_u8(read_u8!(cursor)) {
                Some(x) => x,
                None => AckFreq::Default,
            };
            self.os = match FromPrimitive::from_u8(read_u8!(cursor)) {
                Some(x) => x,
                None => HostOs::Linux,
            };
            self.app = match FromPrimitive::from_u8(read_u8!(cursor)) {
                Some(x) => x,
                None => return ReadResult::Err(ReadError::new("Unknown application.")),
            };
            self.app_ver = read_u8!(cursor);
        });
        ReadResult::Ok
    }
}

/////////////////////////////////
// ObjectHeader

#[derive(Default, Debug, PartialEq)]
pub struct ObjectHeader {
    pub object_id: ObjectId,
    pub num_chunks: ChunkId, // LEB128
    pub ack_req: bool,       // Ack required
    pub object_type: ObjectType,
    pub fields: Vec<ObjectFieldDescription>,
}

const HEADER_ACK_REQUEST_BITMASK: u8 = 0b1000_0000;

impl WireFormat for ObjectHeader {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, TlvType::ObjectHeader, {
            write_u64!(cursor, self.object_id);
            write_u128!(cursor, self.num_chunks as u64);
            match self.ack_req {
                true => write_u8!(cursor, HEADER_ACK_REQUEST_BITMASK),
                false => write_u8!(cursor, 0),
            };
            write_u8!(cursor, self.object_type);
            write_u8!(cursor, self.fields.len() as u8);
            for field in &self.fields {
                field.write(cursor);
            }
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, TlvType::ObjectHeader, {
            self.object_id = read_u64!(cursor);
            self.num_chunks = read_u128!(cursor) as ChunkId;
            self.ack_req = match read_u8!(cursor) {
                HEADER_ACK_REQUEST_BITMASK => true,
                _ => false,
            };
            self.object_type = read_u8!(cursor);
            let mut num_fields = read_u8!(cursor);
            while num_fields > 0 {
                let mut field_description = ObjectFieldDescription::default();
                field_description.read(cursor);
                self.fields.push(field_description);
                num_fields -= 1;
            }
        });
        ReadResult::Ok
    }
}

/////////////////////////////////
// ObjectField

#[derive(Default, Debug, PartialEq)]
pub struct ObjectFieldDescription {
    pub field_type: ObjectFieldType,
    pub length: ChunkId, // in nr. of chunks
}

impl WireFormat for ObjectFieldDescription {
    fn write(&self, cursor: &mut Cursor) {
        write_u8!(cursor, self.field_type);
        write_u128!(cursor, self.length as u64);
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        self.field_type = read_u8!(cursor);
        self.length = read_u128!(cursor) as ChunkId;
        ReadResult::Ok
    }
}

/////////////////////////////////
// ObjectChunk

#[derive(Default, Debug, PartialEq)]
pub struct ObjectChunk {
    pub object_id: ObjectId,
    pub chunk_id: ChunkId, // signed LEB128
    pub more_chunks: bool,
    pub ack_required: bool,
    pub num_enclosed_msgs: u8,
    pub data: Vec<u8>,
}

const MORE_CHUNKS_BITMASK: u16 = 0b1000_0000_0000_0000;
const CHUNK_ACK_REQUEST_BITMASK: u16 = 0b0100_0000_0000_0000;
const CHUNK_SIZE_BITMASK: u16 = 0b0000_0111_1111_1111;

impl WireFormat for ObjectChunk {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, TlvType::ObjectChunk, {
            write_u64!(cursor, self.object_id);
            write_i128!(cursor, self.chunk_id);
            write_u16!(
                cursor,
                {
                    if self.more_chunks {
                        MORE_CHUNKS_BITMASK
                    } else {
                        0
                    }
                } | {
                    if self.ack_required {
                        CHUNK_ACK_REQUEST_BITMASK
                    } else {
                        0
                    }
                } | (self.data.len() + 1) as u16
            );
            write_u8!(cursor, self.num_enclosed_msgs);
            cursor.write(&self.data).expect("Chunk data write failed!");
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, TlvType::ObjectChunk, {
            self.object_id = read_u64!(cursor);
            self.chunk_id = read_i128!(cursor);
            let mut chunksize = read_u16!(cursor);
            self.more_chunks = (chunksize & MORE_CHUNKS_BITMASK) != 0;
            self.ack_required = (chunksize & CHUNK_ACK_REQUEST_BITMASK) != 0;
            self.num_enclosed_msgs = read_u8!(cursor);
            chunksize = (chunksize & CHUNK_SIZE_BITMASK) - 1;
            self.data.reserve(chunksize as usize);
            while chunksize > 0 {
                match cursor.read_u8() {
                    Ok(byte) => self.data.push(byte),
                    Err(err) => return ReadResult::Err(ReadError::new(&err.to_string())),
                }
                chunksize -= 1;
            }
        });
        ReadResult::Ok
    }
}

/////////////////////////////////
// ObjectSkip

#[derive(Default, Debug, PartialEq)]
pub struct ObjectSkip {
    object_id: ObjectId,
    skip_to: ChunkId,
}

impl WireFormat for ObjectSkip {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, TlvType::ObjectSkip, {
            write_u64!(cursor, self.object_id);
            write_i128!(cursor, self.skip_to);
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, TlvType::ObjectSkip, {
            self.object_id = read_u64!(cursor);
            self.skip_to = read_i128!(cursor);
        });
        ReadResult::Ok
    }
}

/////////////////////////////////
// ObjectAck

#[derive(Default, Debug, PartialEq)]
pub struct ObjectAck {
    acknowledged_object_chunks: Vec<(ObjectId, ChunkId)>,
}

impl WireFormat for ObjectAck {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, TlvType::ObjectChunk, {
            write_u8!(cursor, self.acknowledged_object_chunks.len() as u8);
            for chunk in &self.acknowledged_object_chunks {
                write_u64!(cursor, chunk.0);
                write_i128!(cursor, chunk.1);
            }
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, TlvType::ObjectChunk, {
            let num_acks = read_u8!(cursor);
            self.acknowledged_object_chunks.reserve(num_acks as usize);
            while num_acks > 0 {
                self.acknowledged_object_chunks
                    .push((read_u64!(cursor), read_i128!(cursor)));
            }
        });
        ReadResult::Ok
    }
}

/////////////////////////////////
// ErrorMessage

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq)]
#[repr(u8)]
enum ErrorCode {
    None = 0,
    ChecksumError = 4,
    UnsupportedVersion = 5,
    SessionUnknown = 6,
    ObjectAbort = 8,
}

impl Default for ErrorCode {
    fn default() -> Self {
        ErrorCode::None
    }
}

#[derive(Default, Debug, PartialEq)]
pub struct MaxMinSupportedVersion {
    max_ver: Version,
    min_ver: Version,
}

type AbortedObjectIds = Vec<ObjectId>;

#[derive(Debug, PartialEq)]
enum ErrorData {
    UnsupportedVersion(MaxMinSupportedVersion),
    ObjectAbort(AbortedObjectIds),
    None,
}

impl Default for ErrorData {
    fn default() -> Self {
        ErrorData::None
    }
}

#[derive(Default, Debug, PartialEq)]
pub struct ErrorMessage {
    code: ErrorCode,
    detail: ErrorData,
}

impl WireFormat for ErrorMessage {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, TlvType::ObjectChunk, {
            write_u8!(cursor, self.code.to_u8().unwrap());
            match (&self.code, &self.detail) {
                (ErrorCode::UnsupportedVersion, ErrorData::UnsupportedVersion(x)) => {
                    write_u8!(cursor, x.max_ver);
                    write_u8!(cursor, x.min_ver);
                }
                (ErrorCode::ObjectAbort, ErrorData::ObjectAbort(x)) => {
                    write_u8!(cursor, x.len() as u8);
                    for id in x {
                        write_u64!(cursor, *id);
                    }
                }
                (_, _) => {}
            }
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, TlvType::ObjectChunk, {
            self.code = FromPrimitive::from_u8(read_u8!(cursor)).unwrap();
            self.detail = match &self.code {
                UnsupportedVersion => ErrorData::UnsupportedVersion(MaxMinSupportedVersion {
                    max_ver: read_u8!(cursor),
                    min_ver: read_u8!(cursor),
                }),
                ObjectAbort => {
                    let mut result = Vec::new();
                    let mut num_aborted_object_ids = read_u8!(cursor);
                    result.reserve(num_aborted_object_ids as usize);
                    while num_aborted_object_ids > 0 {
                        result.push(read_u64!(cursor));
                        num_aborted_object_ids -= 1;
                    }
                    ErrorData::ObjectAbort(result)
                }
                _ => ErrorData::None,
            }
        });
        ReadResult::Ok
    }
}

/////////////////////////////////
// ObjectAckRequest

#[derive(Default, Debug, PartialEq)]
pub struct ObjectAckRequest {
    req_ack_object_chunks: Vec<(ObjectId, ChunkId)>,
}

impl WireFormat for ObjectAckRequest {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, TlvType::ObjectChunk, {
            write_u8!(cursor, self.req_ack_object_chunks.len() as u8);
            for chunk in &self.req_ack_object_chunks {
                write_u64!(cursor, chunk.0);
                write_i128!(cursor, chunk.1);
            }
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, TlvType::ObjectChunk, {
            let num_acks = read_u8!(cursor);
            self.req_ack_object_chunks.reserve(num_acks as usize);
            while num_acks > 0 {
                self.req_ack_object_chunks
                    .push((read_u64!(cursor), read_i128!(cursor)));
            }
        });
        ReadResult::Ok
    }
}
