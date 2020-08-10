
mod frame;

#[cfg(test)]
mod test;

use frame::*;

pub const CHUNKSIZE: usize = 512; ///maximum chunksize in bytes.

type Stub = String;

//////////////////////////
// Interfaces which define, what an application can send

/// This is a description what can be sent and received by an application. 
/// While using a `Connection` the application will provide callbacks to 
/// receive and provide chunks of this object. 
pub trait Object<T: ObjectField> {
    fn get_type(&self) -> ObjectType;

    /// size in chunks.
    fn get_size(&self) -> usize;

    /// An Object must contain at least one field.
    fn get_field(&self, index: u32) -> T;

    /// Must be >= 1.
    fn get_field_count(&self) -> usize;
}

/// Application may use this extensively to structure their Objects. 
/// The actual content of a field must be offered by Object.get_chunk.
pub trait ObjectField {
    fn get_type(&self) -> ObjectFieldType;

    /// size in chunks.
    fn get_size() -> usize;
}

//////////////////////////
// The Connection handlers.

pub struct Connection<T: ObjectField, U: Object<T>> {
    /// Called by the transport layer to notify the application 
    /// about new receiving Objects. 
    /// The application returns None, if it is not interested in the Object. 
    /// Otherwise the application returns a function allowing the transport 
    /// layer to pass the chunks of this Object on.
    accept_callback: fn (receiver: ObjectReceiver<T, U>) -> 
        Option<fn (chunk: &Vec<u8>, id: ChunkId) -> ()>,
}

impl<T: ObjectField, U: Object<T>> Connection<T, U>{
    pub fn new(
        accept_callback: fn (receiver: ObjectReceiver<T, U>) -> 
        Option<fn (chunk: &Vec<u8>, id: ChunkId) -> ()>,
        ) -> Connection<T, U> {

        Connection::<T, U>{
            accept_callback: accept_callback,
        }
    }

    /// nonblocking.
    ///
    /// start off with chunk number `start`
    pub fn send(&self, object: U, start: ChunkId) -> ObjectSender {
        ObjectSender{}
    }
}

/// Handler for an Object which is in sending transmission. 
/// Chunks are provided to the transport layer via Connection.send(_, _, get_chunk)
pub struct ObjectSender {
}

impl ObjectSender {
    pub fn abort_sending(&mut self) {
        // TODO
    }

    pub fn skip_to(&mut self, chunk: ChunkId) {
        // TODO
    }

    /// optionally TODO:
    pub fn progress(&self) -> f64 {
        0.0
    }
}

pub struct ObjectReceiver<T: ObjectField, U: Object<T>> {
}

impl<T: ObjectField, U: Object<T>> ObjectReceiver<T, U> {
    /// Used to get metadata about the object.
    pub fn get_object(&self) -> U {
        // TODO
    }

    /// instead of returning None through the Connection.accept_callback 
    /// lambda function, the application may also abort receiving at 
    /// a later point. 
    pub fn abort_receiving(&mut self) {
        // TODO
    }
}

