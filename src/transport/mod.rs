
mod frame;

#[cfg(test)]
mod test;

use frame::*;
use std::net::SocketAddr;

pub const CHUNKSIZE: usize = 512; ///maximum chunksize in bytes.

//////////////////////////
// Interfaces which define, what an application can send

/// This is a description what can be sent and received by an application. 
/// While using a `Connection` the application will provide callbacks to 
/// receive and provide chunks of this object. 
pub trait Object<T: ObjectField> {
    fn get_type(&self) -> ObjectType;

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
    fn get_size(&self) -> usize;
}

//////////////////////////
// Object and ObjectField implementation used by the transport 
// layer to offer information to the application.

struct ReceivingObject {
}

impl Object<ReceivingObjectField> for ReceivingObject {
    fn get_type(&self) -> ObjectType {
        todo!();
    }
    
    fn get_field(&self, index: u32) -> ReceivingObjectField {
        todo!();
    }

    fn get_field_count(&self) -> usize {
        todo!();
    }
}

struct ReceivingObjectField {
}

impl ObjectField for ReceivingObjectField {
    fn get_type(&self) -> ObjectFieldType {
        todo!();
    }

    fn get_size(&self) -> usize {
        todo!();
    }
}

//////////////////////////
// Establishing a connection.

/// Is called by the transport layer to inform the application 
/// about new connections. The server-application then returns its
/// ObjectListener for that connection. 
pub type ConnectionListener<T: ObjectField, U: Object<T>> = fn (incoming: Connection<T,U>) -> ObjectListener<T,U>;

/// Used by servers to listen for incoming connections.
/// Install a ConnectionListener to be called for each new 
/// incoming connection. Will listen at `bind`.
/// Non-blocking.
pub fn server_listen<T: ObjectField, U: Object<T>>(bind: SocketAddr, callback: ConnectionListener<T, U>) -> () {
    todo!();
}

/// Called by an application to create a `Connection`. Will bind 
/// to `0.0.0.0:random` as src addr.
/// Non-blocking as it only creates state. The `Connection` will then 
/// be established with handshake and everything while being granted 
/// cpu_time by `Connection.grant_cpu()`.
pub fn client_connect<T: ObjectField, U: Object<T>>(dest: SocketAddr, accept_callback: ObjectListener<T,U>, timeout_callback: TimeoutListener) -> Connection<T, U> {
    Connection::<T, U>{
            accept_callback: accept_callback,
            timeout_callback: timeout_callback,
            placeholder_state: std::marker::PhantomData::default(),
            placeholder_state_2: std::marker::PhantomData::default(),
    }
}

//////////////////////////
// The Connection handlers.

/// Will be called by the transport layer to notify the application 
/// about new receiving Objects. 
/// The application returns None, if it is not interested in the Object. 
/// Otherwise the application returns its ChunkListener for that Object.
pub type ObjectListener<T: ObjectField, U: Object<T>> = 
    fn (receiver: ObjectReceiver<T, U>) -> Option<ChunkListener>;

/// Will be called by the transport layer to pass a chunk on to 
/// the application. There is one ChunkListener per Object.
pub type ChunkListener = fn (chunk: &Vec<u8>, id: ChunkId) -> ();

/// Will be called by the transport layer to inform the application 
/// about a timeout of a connection. 
pub type TimeoutListener = fn () -> ();

pub struct Connection<T: ObjectField, U: Object<T>> {
    accept_callback: ObjectListener<T,U>,
    timeout_callback: TimeoutListener,

    /// required to suppress "unused" errors for T and U
    placeholder_state: std::marker::PhantomData<U>,
    placeholder_state_2: std::marker::PhantomData<T>,
}

impl<T: ObjectField, U: Object<T>> Connection<T, U>{
    /// nonblocking.
    ///
    /// start off with chunk number `start`
    pub fn send(&self, object: U, start: ChunkId) -> ObjectSender {
        ObjectSender{}
    }

    /// Should return within about 0.1s to allow the application to interact
    /// with the user still. 
    /// Must be called by the application in its main loop.
    pub fn grant_cpu(&mut self) {
        todo!();
    }
}

/// Handler for an Object which is in sending transmission. 
/// Chunks are provided to the transport layer via Connection.send(_, _, get_chunk)
pub struct ObjectSender {
}

impl ObjectSender {
    pub fn abort_sending(&mut self) {
        todo!();
    }

    pub fn skip_to(&mut self, chunk: ChunkId) {
        todo!();
    }

    pub fn progress(&self) -> f64 {
        unimplemented!();
    }
}

pub struct ObjectReceiver<T: ObjectField, U: Object<T>> {
    /// required to suppress "unused" errors for T and U
    placeholder_state: std::marker::PhantomData<U>,
    placeholder_state_2: std::marker::PhantomData<T>,
}

impl<T: ObjectField, U: Object<T>> ObjectReceiver<T, U> {
    /// Used to get metadata about the object.
    pub fn get_object(&self) -> U {
        todo!();
    }

    /// instead of returning None through the Connection.accept_callback 
    /// lambda function, the application may also abort receiving at 
    /// a later point. 
    pub fn abort_receiving(&mut self) {
        todo!();
    }
}

