use std::vec;
use crate::common::*;
use byteorder::{NetworkEndian, NativeEndian, WriteBytesExt};
use std::io::{Write, Seek};
use leb128;
use std::io::SeekFrom;
use crate::common::fnv1a32;

/////////////////////////////////
// SessionId

pub type SessionId = u64;
pub type ObjectId = u64;
pub type ChunkId = u64;

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
            match tlv {
                TransportTlv::ObjectHeader(x) => {
                    x.serialize(cursor);
                }
            }
        }
        let end = cursor.position();
        let checksum = fnv1a32::Fnv32a::hash(cursor, start, end);
        cursor.write_u32::<NetworkEndian>(checksum);
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

/////////////////////////////////
// ObjectHeader

pub struct ObjectHeader {
    object_id: ObjectId,
    n_chunks: ChunkId, // LEB128
    ack_req: bool, // Ack required
    object_type: u8,
    fields: Vec<ObjectField>
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
}

/////////////////////////////////
// ObjectField

/// An ObjectField described by `ObjectField` contains multiple ObjectFieldParts on higher layers

pub type ObjectFieldPart = Vec<u8>;

pub struct ObjectField {
    field_type: u8,
    length: ChunkId // in nr. of chunks
}

impl Serializable for ObjectField {
    fn serialize(&self, cursor: &mut Cursor) {
        cursor.write_u8(self.field_type);
        leb128::write::unsigned(cursor, self.length);
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
    content: Vec<ObjectFieldPart>
}
