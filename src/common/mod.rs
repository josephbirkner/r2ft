use std::error::Error;
use std::io;
use std::fmt;
pub use std::str::FromStr;

/////////////////////////////////
// Use third-party FNV hash

pub mod fnv1a32;

/////////////////////////////////
// Basic Types

pub type Cursor = io::Cursor<Vec<u8>>;

/////////////////////////////////
// SerializationError

#[derive(Debug)]
pub struct SerializationError {
    what: String
}

impl SerializationError {
    pub fn new(msg: &str) -> SerializationError {
        SerializationError{what: msg.to_string()}
    }
}

impl fmt::Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.what)
    }
}

impl Error for SerializationError {
    fn description(&self) -> &str {
        &self.what
    }
}

pub enum SerializationResult {
    Ok, Err(SerializationError)
}

/////////////////////////////////
// Serializable

pub trait Serializable {
    fn serialize(&self, cursor: &mut Cursor);
    fn deserialize(&mut self, cursor: &mut Cursor) -> SerializationResult;
}

/////////////////////////////////
// Deserialization macros

macro_rules! read_u8 {
    ($cursor:ident) => {
        match $cursor.read_u8() {
            Ok(x) => x,
            Err(e) => return SerializationResult::Err(SerializationError::new(&e.to_string()))
        };
    };
}

macro_rules! read_u16 {
    ($cursor:ident) => {
        match $cursor.read_u16::<NetworkEndian>() {
            Ok(x) => x,
            Err(e) => return SerializationResult::Err(SerializationError::new(&e.to_string()))
        };
    };
}

macro_rules! read_u32 {
    ($cursor:ident) => {
        match $cursor.read_u32::<NetworkEndian>() {
            Ok(x) => x,
            Err(e) => return SerializationResult::Err(SerializationError::new(&e.to_string()))
        };
    };
}

macro_rules! read_u64 {
    ($cursor:ident) => {
        match $cursor.read_u64::<NetworkEndian>() {
            Ok(x) => x,
            Err(e) => return SerializationResult::Err(SerializationError::new(&e.to_string()))
        };
    };
}

macro_rules! read_leb128 {
    ($cursor:ident) => {
        match leb128::read::unsigned($cursor) {
            Ok(x) => x,
            Err(e) => return SerializationResult::Err(SerializationError::new(&e.to_string()))
        };
    };
}

macro_rules! write_u8 {
    ($cursor:ident, $value:expr) => {
        $cursor.write_u8($value).expect("write_u8: Failed.")
    };
}

macro_rules! write_u16 {
    ($cursor:ident, $value:expr) => {
        $cursor.write_u16::<NetworkEndian>($value).expect("write_u16: Failed.")
    };
}

macro_rules! write_u32 {
    ($cursor:ident, $value:expr) => {
        $cursor.write_u32::<NetworkEndian>($value).expect("write_u32: Failed.")
    };
}

macro_rules! write_u64 {
    ($cursor:ident, $value:expr) => {
        $cursor.write_u64::<NetworkEndian>($value).expect("write_u64: Failed.")
    };
}

macro_rules! write_leb128 {
    ($cursor:ident, $value:expr) => {
        leb128::write::unsigned($cursor, $value).expect("write_leb128: Failed.")
    };
}
