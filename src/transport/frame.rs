use std::vec;
use crate::common::*;
use byteorder::{NetworkEndian, NativeEndian, WriteBytesExt};
use std::io::{Write, Seek};
use leb128;
use std::io::SeekFrom;

/////////////////////////////////
// SessionId

pub type SessionId = u64;

/////////////////////////////////
// MessageFrame

pub struct MessageFrame {
    version: u8,
    sid: SessionId,
    tlvs: Vec<TransportTlv>,
    checksum: u32,
}

impl Serializable for MessageFrame {
    fn serialize(&self, cursor: &mut Cursor) {
        cursor.write_u8(self.version);
        cursor.write_u64::<NetworkEndian>(self.sid);
        for tlv in &self.tlvs {
            match tlv {
                TransportTlv::ObjectHeader(x) => {
                    x.serialize(cursor);
                }
            }
        }
        cursor.write_u32::<NetworkEndian>(self.checksum);
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
    object_id: u64,
    n_chunks: u64, // LEB128
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
            true => cursor.write_u8(0x80),
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

pub struct ObjectField {
    field_type: u8,
    length: u64,
}

impl Serializable for ObjectField {
    fn serialize(&self, cursor: &mut Cursor) {
        cursor.write_u8(self.field_type);
        leb128::write::unsigned(cursor, self.length);
    }
}
