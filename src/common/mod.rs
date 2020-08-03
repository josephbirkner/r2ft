#![feature(macro_rules)]

use std::vec;
use std::io;
pub use std::str::FromStr;

pub mod fnv1a32;

pub type Cursor = io::Cursor<Vec<u8>>;

pub enum SerializationResult {
    Ok, Err(io::Error)
}

pub trait Serializable {
    fn serialize(&mut self, cursor: &mut Cursor);
    fn deserialize(&mut self, cursor: &mut Cursor) -> SerializationResult;
}

macro_rules! read_u8 {
    ($cursor:ident) => {match cursor.read_u8() { Ok(x) => x, Err(e) => return SerializationResult(Err(e)) };};
}

macro_rules! read_u16 {
    ($cursor:ident) => {match cursor.read_u16() { Ok(x) => x, Err(e) => return SerializationResult(Err(e)) };};
}

macro_rules! read_u32 {
    ($cursor:ident) => {match cursor.read_u32() { Ok(x) => x, Err(e) => return SerializationResult(Err(e)) };};
}

macro_rules! read_u64 {
    ($cursor:ident) => {match cursor.read_u64() { Ok(x) => x, Err(e) => return SerializationResult(Err(e)) };};
}

macro_rules! read_leb128 {
    ($cursor:ident) => {match leb128::read::unsigned(cursor) { Ok(x) => x, Err(e) => return SerializationResult(Err(e))}; };
}
