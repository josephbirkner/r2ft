use crate::transport::frame::*;

//////////////////////////
// Object metatype

/// This is a description what can be sent and received by an application.
/// While using a `Connection` the application will provide callbacks to
/// receive and provide chunks of this object.
pub struct Object {
    object_type: ObjectType,
    object_id: ObjectId,
    fields: Vec<ObjectFieldDescription>,
    transmission_finished_callback: TransmissionFinishedListener,
}

/// Called by connection while it is working
/// on an ObjectSendJob.
pub type ChunkProvider = fn (chunk_id: ChunkId) -> Vec<u8>;

/// Called by connection once transfer is done.
pub type TransmissionFinishedListener = fn() -> ();

//////////////////////////
// ObjectSendJob

/// Handler for an Object which is in sending transmission.
/// Chunks are provided to the transport layer via Connection.send(_, _, get_chunk)
pub struct ObjectSendJob {
    /// Abort sending by setting this flag to true.
    pub abort: bool,
    /// Use this field to indicate at which chunk id
    /// the transmission should proceed.
    pub next_chunk: ChunkId,
    /// Object instance that was passed to new()
    object_in_transfer: Object,
    /// Callback which is used by the connection to retrieve chunks.
    get_chunk_callback: ChunkProvider,
}

impl ObjectSendJob {
    pub fn new(obj: &Object, chunk_getter: ChunkProvider) -> Self {
        todo!();
    }

    pub fn object_type(&self) -> ObjectType {
        self.object_in_transfer.object_type
    }

    pub fn object_id(&self) -> ObjectId {
        self.object_in_transfer.object_id
    }
}

//////////////////////////
// ObjectReceiveJob

/// Will be called by the transport layer to pass a chunk on to
/// the application. There is one ChunkListener per Object.
pub type ChunkListener = fn (chunk: &Vec<u8>, id: ChunkId) -> ();

pub struct ObjectReceiveJob {
    pub chunk_received_callback: ChunkListener,
    /// Metadata about the object.
    pub object: Object,
    /// Abort receiving by setting this flag to true.
    pub abort: bool
}
