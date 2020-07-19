use std::vec;
use std::io;

pub type Cursor = io::Cursor<Vec<u8>>;

pub struct Error {
    what: String
}

pub trait Serializable {
    fn serialize(&self, cursor: &mut Cursor);
}

pub trait Deserializable<T> {
    fn deserialize(&self, cursor: &mut Cursor) -> Result<T, Error>;
}
