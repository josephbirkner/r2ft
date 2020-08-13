use std::error::Error;
use std::fmt;
use std::io;
pub use std::str::FromStr;

/////////////////////////////////
// Use third-party FNV hash

pub mod fnv1a32;

/////////////////////////////////
// Custom UdpSocket

pub mod mtu;
pub mod udp;

/////////////////////////////////
// Basic Types

pub type Cursor = io::Cursor<Vec<u8>>;

/////////////////////////////////
// ReadError

#[derive(Debug)]
pub struct ReadError {
    what: String,
}

impl ReadError {
    pub fn new(msg: &str) -> ReadError {
        ReadError {
            what: msg.to_string(),
        }
    }
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.what)
    }
}

impl Error for ReadError {
    fn description(&self) -> &str {
        &self.what
    }
}

pub enum ReadResult {
    Ok,
    Err(ReadError),
}

/////////////////////////////////
// WireFormat trait

pub trait WireFormat {
    fn write(&self, cursor: &mut Cursor);
    fn read(&mut self, cursor: &mut Cursor) -> ReadResult;
}

/////////////////////////////////
// Deserialization macros

macro_rules! read_u8 {
    ($cursor:ident) => {
        match $cursor.read_u8() {
            Ok(x) => x,
            Err(e) => return ReadResult::Err(ReadError::new(&e.to_string())),
        };
    };
}

macro_rules! read_u16 {
    ($cursor:ident) => {
        match $cursor.read_u16::<NetworkEndian>() {
            Ok(x) => x,
            Err(e) => return ReadResult::Err(ReadError::new(&e.to_string())),
        };
    };
}

macro_rules! read_u32 {
    ($cursor:ident) => {
        match $cursor.read_u32::<NetworkEndian>() {
            Ok(x) => x,
            Err(e) => return ReadResult::Err(ReadError::new(&e.to_string())),
        };
    };
}

macro_rules! read_u64 {
    ($cursor:ident) => {
        match $cursor.read_u64::<NetworkEndian>() {
            Ok(x) => x,
            Err(e) => return ReadResult::Err(ReadError::new(&e.to_string())),
        };
    };
}

macro_rules! read_u128 {
    ($cursor:ident) => {
        match leb128::read::unsigned($cursor) {
            Ok(x) => x,
            Err(e) => return ReadResult::Err(ReadError::new(&e.to_string())),
        };
    };
}

macro_rules! read_i128 {
    ($cursor:ident) => {
        match leb128::read::signed($cursor) {
            Ok(x) => x,
            Err(e) => return ReadResult::Err(ReadError::new(&e.to_string())),
        };
    };
}

macro_rules! read_tlv {
    ($cursor:ident, $type_code:expr, $read_block:block) => {
        assert_eq!(read_u8!($cursor), $type_code as u8);
        let length = read_u16!($cursor) as u64;
        let mut final_length = $cursor.position();
        $read_block
        // Validate length field
        final_length = $cursor.position() - final_length;
        if length != final_length {
            return ReadResult::Err(ReadError::new("Object header length mismatch!"));
        }
    };
}

macro_rules! read_str {
    ($cursor:ident) => {{
        let mut buf_len = read_u128!($cursor);
        let mut buf = Vec::new();
        buf.reserve(buf_len as usize);
        while buf_len > 0 {
            buf.push(read_u8!($cursor));
            buf_len -= 1;
        }
        match String::from_utf8(buf) {
            Ok(val) => val,
            _ => return ReadResult::Err(ReadError::new("")),
        }
    }};
}

/////////////////////////////////
// Serialization macros

macro_rules! write_u8 {
    ($cursor:ident, $value:expr) => {
        $cursor.write_u8($value).expect("write_u8: Failed.")
    };
}

macro_rules! write_u16 {
    ($cursor:ident, $value:expr) => {
        $cursor
            .write_u16::<NetworkEndian>($value)
            .expect("write_u16: Failed.")
    };
}

macro_rules! write_u32 {
    ($cursor:ident, $value:expr) => {
        $cursor
            .write_u32::<NetworkEndian>($value)
            .expect("write_u32: Failed.")
    };
}

macro_rules! write_u64 {
    ($cursor:ident, $value:expr) => {
        $cursor
            .write_u64::<NetworkEndian>($value)
            .expect("write_u64: Failed.")
    };
}

macro_rules! write_u128 {
    ($cursor:ident, $value:expr) => {
        leb128::write::unsigned($cursor, $value).expect("write_u128: Failed.")
    };
}

macro_rules! write_i128 {
    ($cursor:ident, $value:expr) => {
        leb128::write::signed($cursor, $value).expect("write_i128: Failed.")
    };
}

macro_rules! write_tlv {
    ($cursor:ident, $type_code:expr, $write_block:block) => {{
        write_u8!($cursor, $type_code as u8);
        let mut length = $cursor.position();
        write_u16!($cursor, 0);
        $write_block
        length = $cursor.position() - length;
        $cursor.seek(SeekFrom::Current(-(length as i64))).expect("seek failed.");
        length -= 2; // -2 bc. of 2B length-field length
        write_u16!($cursor, length as u16);
        $cursor.seek(SeekFrom::Current(length as i64)).expect("seek failed.");
    }};
}

macro_rules! write_str {
    ($cursor:ident, $value:expr) => {
        let buf = $value.clone().into_bytes();
        write_u128!($cursor, buf.len() as u64);
        if $cursor.write(&buf).is_err() {
            todo!("Implement error handling.")
        }
    };
}
