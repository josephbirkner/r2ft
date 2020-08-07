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

pub struct MessageFrame {
    version: u8,
    sid: SessionId,
    tlvs: Vec<TransportTlv>,
}

impl Serializable for MessageFrame {
    fn serialize(&self, cursor: &mut Cursor) {
        let start = cursor.position();
        cursor.write_u8(self.version);
        cursor.write_u64::<NetworkEndian>(self.sid);
        cursor.write_u8(self.tlvs.len() as u8);
        for tlv in &self.tlvs {
            tlv.serialize(cursor);
        }
        let end = cursor.position();
        let checksum = fnv1a32::Fnv32a::hash(cursor, start, end);
        cursor.write_u32::<NetworkEndian>(checksum);
    }

    fn deserialize(&mut self, cursor: &mut Cursor) -> SerializationResult {
        self.version = read_u8!(cursor);
        self.sid = read_u64!(cursor);
        self.tlvs = Vec::new();
        let mut num_tlvs = read_u8!(cursor);
        while num_tlvs > 0 {
            let tlv_type = match cursor.read_u8() {
                Ok(typeCode) => typeCode,
                Err(_e) => return SerializationResult::Ok // Err in this case means eof, that's ok
            };
            let mut tlv = match tlv_type {
                tlv_type if tlv_type == (TransportTlvTypeCode::ObjectHeader as u8) => TransportTlv::ObjectHeader(ObjectHeader::default()),
                _ => return SerializationResult::Err(SerializationError::new("Unknown object type code!"))
            };
            tlv.deserialize(cursor);
            self.tlvs.push(tlv);
            num_tlvs -= 1;
        }

        // FIXME: Compare hash!
        SerializationResult::Ok
    }
}

/////////////////////////////////
// TransportTlv

pub enum TransportTlv {
    ObjectHeader(ObjectHeader)
}

enum TransportTlvTypeCode {
    ObjectHeader = 0x51
}

impl Serializable for TransportTlv {
    fn serialize(&self, cursor: &mut Cursor) {
        match self {
            TransportTlv::ObjectHeader(x) => {
                x.serialize(cursor);
            }
        }
    }

    fn deserialize(&mut self, cursor: &mut Cursor) -> SerializationResult {
        match self {
            TransportTlv::ObjectHeader(x) => {
                x.deserialize(cursor);
            }
        }
        SerializationResult::Ok
    }
}

/////////////////////////////////
// ObjectHeader

#[derive(Default)]
pub struct ObjectHeader {
    object_id: ObjectId,
    n_chunks: ChunkId, // LEB128
    ack_req: bool, // Ack required
    object_type: ObjectType,
    fields: Vec<ObjectFieldDescription>
}

impl Serializable for ObjectHeader {
    fn serialize(&self, cursor: &mut Cursor) {
        // Write TLV stub - reserve 16b for content length
        cursor.write_u8(TransportTlvTypeCode::ObjectHeader as u8);
        let mut length = cursor.position();
        cursor.write_u16::<NetworkEndian>(0);

        // Write object header content
        cursor.write_u64::<NetworkEndian>(self.object_id);
        leb128::write::unsigned(cursor, self.n_chunks);
        match self.ack_req {
            true => cursor.write_u8(0b1000_0000),
            false => cursor.write_u8(0x00)
        };
        cursor.write_u8(self.object_type);
        cursor.write_u8(self.fields.len() as u8);
        for field in &self.fields {
            field.serialize(cursor);
        }

        // Determine & write length field
        length -= cursor.position();
        cursor.seek(SeekFrom::Current(-(length as i64)));
        cursor.write_u16::<NetworkEndian>((length - 2) as u16); // -2 bc. of length-field
        cursor.seek(SeekFrom::Current(length as i64));
    }

    fn deserialize(&mut self, cursor: &mut Cursor) -> SerializationResult {
        // NOTE: TLV Type is already parsed!
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
            return SerializationResult::Err(SerializationError::new("Unknown object type code!"));
        }
        SerializationResult::Ok
    }
}

/////////////////////////////////
// ObjectField

/// An ObjectField described by `ObjectField` contains multiple ObjectFieldContents on higher layers
pub type ObjectFieldContent = Vec<u8>;

#[derive(Default)]
pub struct ObjectFieldDescription {
    field_type: ObjectFieldType,
    length: ChunkId // in nr. of chunks
}

impl Serializable for ObjectFieldDescription{
    fn serialize(&self, cursor: &mut Cursor) {
        cursor.write_u8(self.field_type);
        leb128::write::unsigned(cursor, self.length);
    }

    fn deserialize(&mut self, cursor: &mut Cursor) -> SerializationResult {
        self.field_type = read_u8!(cursor);
        self.length = read_leb128!(cursor) as ChunkId;
        SerializationResult::Ok
    }
}

/////////////////////////////////
// ObjectChunk

pub struct ObjectChunk {
    object_id: ObjectId,
    chunk_id: ChunkId,
    last_chunk: bool,
    ack_required: bool,
    size_following_chunk: u16, // 11 bit ???
    content: Vec<ObjectFieldContent>
}
