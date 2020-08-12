use crate::transport::frame::*;

//////////////////////////
// Object metatype

/// This is a description what can be sent and received by an application.
/// While using a `Connection` the application will provide callbacks to
/// receive and provide chunks of this object.
pub struct Object {
    pub object_type: ObjectType,
    pub object_id: ObjectId,
    pub fields: Vec<ObjectFieldDescription>,
    pub transmission_finished_callback: Box<TransmissionFinishedListener>,
}

/// Called by transport layer while it is working
/// on an ObjectSendJob.
/// Number of application tlvs in this chunk are returned in as second
/// element of the tuple.
pub type ChunkProvider = dyn FnMut (ChunkId) -> (Vec<u8>, u8);

/// Called by connection once transfer is done.
pub type TransmissionFinishedListener = dyn FnMut() -> ();

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
    get_chunk_callback: Box<ChunkProvider>,
}

impl ObjectSendJob {
    pub fn new(obj: Object, chunk_getter: Box<ChunkProvider>) -> Self {
        ObjectSendJob {
            abort: false,
            next_chunk: -1,
            object_in_transfer: obj,
            get_chunk_callback: chunk_getter
        }
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
/// Number of application tlvs in this chunk in `nr_tlv`.
pub type ChunkListener = fn (chunk: &Vec<u8>, id: ChunkId, nr_tlv: u8) -> ();

pub struct ObjectReceiveJob {
    pub chunk_received_callback: ChunkListener,
    /// Metadata about the object.
    pub object: Object,
    /// Abort receiving by setting this flag to true.
    pub abort: bool
}
