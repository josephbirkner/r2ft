use byteorder::{NetworkEndian, WriteBytesExt, ReadBytesExt};
use std::io::{Seek, SeekFrom};
use leb128;
use crate::common::fnv1a32;
use crate::common::*;

/////////////////////////////////
// Basic Types

pub type SessionId = u64;
pub type ObjectId = u64;
pub type ChunkId = u64;
pub type ObjectType = u8;
pub type ObjectFieldType = u8;

/////////////////////////////////
// MessageFrame

#[derive(Default, Debug, PartialEq)]
pub struct MessageFrame {
    pub version: u8,
    pub sid: SessionId,
    pub tlvs: Vec<TransportTlv>,
}

impl Serializable for MessageFrame {
    fn serialize(&self, cursor: &mut Cursor) {
        let start = cursor.position();
        write_u8!(cursor, self.version);
        write_u64!(cursor, self.sid);
        write_u8!(cursor, self.tlvs.len() as u8);
        for tlv in &self.tlvs {
            tlv.serialize(cursor);
        }
        let end = cursor.position();
        let checksum = fnv1a32::Fnv32a::hash(cursor, start, end);
        write_u32!(cursor, checksum);
    }

    fn deserialize(&mut self, cursor: &mut Cursor) -> SerializationResult {
        let start = cursor.position();
        self.version = read_u8!(cursor);
        self.sid = read_u64!(cursor);
        self.tlvs = Vec::new();
        let mut num_tlvs = read_u8!(cursor);
        while num_tlvs > 0 {
            let tlv_type = read_u8!(cursor);
            cursor.seek(SeekFrom::Current(-1)); // TLV will read type again.
            let mut tlv = match tlv_type {
                tlv_type if tlv_type == (TransportTlvTypeCode::ObjectHeader as u8) => TransportTlv::ObjectHeader(ObjectHeader::default()),
                _ => return SerializationResult::Err(SerializationError::new(format!("Unknown object type code {}!", tlv_type).as_str()))
            };
            tlv.deserialize(cursor);
            self.tlvs.push(tlv);
            num_tlvs -= 1;
        }
        let end = cursor.position();
        let checksum = fnv1a32::Fnv32a::hash(cursor, start, end);
        let advertised_checksum = read_u32!(cursor);
        if checksum != advertised_checksum {
            return SerializationResult::Err(SerializationError::new("Checksum error!"))
        }
        SerializationResult::Ok
    }
}

/////////////////////////////////
// TransportTlv

#[derive(Debug, PartialEq)]
pub enum TransportTlv {
    ObjectHeader(ObjectHeader)
}

enum TransportTlvTypeCode {
    ObjectHeader = 0x51
}

impl Serializable for TransportTlv {
    fn serialize(&self, cursor: &mut Cursor) {
        match self {
            TransportTlv::ObjectHeader(x) => x.serialize(cursor)
        }
    }

    fn deserialize(&mut self, cursor: &mut Cursor) -> SerializationResult {
        return match self {
            TransportTlv::ObjectHeader(x) => x.deserialize(cursor)
        };
    }
}

/////////////////////////////////
// ObjectHeader

#[derive(Default, Debug, PartialEq)]
pub struct ObjectHeader {
    pub object_id: ObjectId,
    pub n_chunks: ChunkId, // LEB128
    pub ack_req: bool, // Ack required
    pub object_type: ObjectType,
    pub fields: Vec<ObjectFieldDescription>
}

impl Serializable for ObjectHeader {
    fn serialize(&self, cursor: &mut Cursor) {
        // Write TLV stub - reserve 16b for content length
        write_u8!(cursor, TransportTlvTypeCode::ObjectHeader as u8);
        let mut length = cursor.position();
        write_u16!(cursor, 0);

        // Write object header content
        write_u64!(cursor, self.object_id);
        write_leb128!(cursor, self.n_chunks);
        match self.ack_req {
            true => write_u8!(cursor, 0b1000_0000),
            false => write_u8!(cursor, 0x00)
        };
        write_u8!(cursor, self.object_type);
        write_u8!(cursor, self.fields.len() as u8);
        for field in &self.fields {
            field.serialize(cursor);
        }

        // Determine & write length field
        length = cursor.position() - length;
        cursor.seek(SeekFrom::Current(-(length as i64))).expect("seek failed.");
        length -= 2; // -2 bc. of 2B length-field length
        write_u16!(cursor, length as u16);
        cursor.seek(SeekFrom::Current(length as i64)).expect("seek failed.");;
    }

    fn deserialize(&mut self, cursor: &mut Cursor) -> SerializationResult {
        assert_eq!(read_u8!(cursor), TransportTlvTypeCode::ObjectHeader as u8);
        let length = read_u16!(cursor) as u64;
        let pos = cursor.position();

        // Read object header content
        self.object_id = read_u64!(cursor);
        self.n_chunks = read_leb128!(cursor);
        self.ack_req = match read_u8!(cursor) {
            0b1000_0000 => true,
            _ => false,
        };
        self.object_type = read_u8!(cursor);
        let mut num_fields = read_u8!(cursor);
        while num_fields > 0 {
            let mut field_description = ObjectFieldDescription::default();
            field_description.deserialize(cursor);
            self.fields.push(field_description);
            num_fields -= 1;
        }

        // Determine & write length field
        let final_length = cursor.position() - pos;
        if length != final_length {
            return SerializationResult::Err(SerializationError::new("Object header length mismatch!"));
        }
        SerializationResult::Ok
    }
}

/////////////////////////////////
// ObjectField

/// An ObjectField described by `ObjectField` contains multiple ObjectFieldContents on higher layers
pub type ObjectFieldContent = Vec<u8>;

#[derive(Default, Debug, PartialEq)]
pub struct ObjectFieldDescription {
    pub field_type: ObjectFieldType,
    pub length: ChunkId // in nr. of chunks
}

impl Serializable for ObjectFieldDescription{
    fn serialize(&self, cursor: &mut Cursor) {
        write_u8!(cursor, self.field_type);
        write_leb128!(cursor, self.length);
    }

    fn deserialize(&mut self, cursor: &mut Cursor) -> SerializationResult {
        self.field_type = read_u8!(cursor);
        self.length = read_leb128!(cursor) as ChunkId;
        SerializationResult::Ok
    }
}

/////////////////////////////////
// ObjectChunk

#[derive(Default, Debug, PartialEq)]
pub struct ObjectChunk {
    pub object_id: ObjectId,
    pub chunk_id: ChunkId,
    pub last_chunk: bool,
    pub ack_required: bool,
    pub size_following_chunk: u16, // 11 bit ???
    pub content: Vec<ObjectFieldContent>
}
