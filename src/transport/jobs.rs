use crate::transport::frame::*;
use crate::transport::connection::*;
use crate::transport::common;

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
pub type ChunkProvider = dyn FnMut(ChunkId) -> (Vec<u8>, u8);

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
    /// the transmission should proceed. Not to be changed
    /// by the SendJob itself.
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
            get_chunk_callback: chunk_getter,
        }
    }

    pub fn object_type(&self) -> ObjectType {
        self.object_in_transfer.object_type
    }

    pub fn object_id(&self) -> ObjectId {
        self.object_in_transfer.object_id
    }

    /// TODO
    pub(super) fn ack_required(&self) -> bool {
        false
    }

    fn count_chunks(&self) -> ChunkId {
        let n_chunks: ChunkId = self.object_in_transfer.fields.iter().map(|field| field.length).sum();
        return n_chunks;
    }

    /// wether self has a chunk after next_chunk
    pub(super) fn has_next(&self) -> bool {
        let n_chunks: ChunkId = self.count_chunks();
        let last_chunk = n_chunks - 1;
        return self.next_chunk < last_chunk;
    }

    fn send_o_header(&mut self, mut session: &EstablishedState) -> MessageFrame {
        // build ObjectChunk message
        let mut msg: MessageFrame = MessageFrame::default();
        msg.sid = session.sessionid;
        msg.version = common::PROTOCOL_VERSION;
        msg.tlvs = Vec::new();
        let oh: ObjectHeader = ObjectHeader {
            object_id: self.object_id(),
            num_chunks: self.count_chunks(),
            ack_req: self.ack_required(),
            object_type: self.object_type(),
            fields: self.object_in_transfer.fields.clone(),
        };
        msg.tlvs.push(Tlv::ObjectHeader(oh));

        return msg;
    }

    fn send_o_chunk(&mut self, mut session: &EstablishedState) -> MessageFrame {
        let (chunk, n_tlvs) = (self.get_chunk_callback)(self.next_chunk);

        // build ObjectChunk message
        let mut msg: MessageFrame = MessageFrame::default();
        msg.sid = session.sessionid;
        msg.version = common::PROTOCOL_VERSION;
        msg.tlvs = Vec::new();
        let oc: ObjectChunk = ObjectChunk {
            object_id: self.object_id(),
            chunk_id: self.next_chunk,
            more_chunks: self.has_next(),
            ack_required: self.ack_required(),
            num_enclosed_msgs: n_tlvs,
            data: chunk,
        };
        msg.tlvs.push(Tlv::ObjectChunk(oc));

        return msg;
    }

    /// advances the state for having sent the returned chunk
    pub(super) fn send_next(&mut self, session: &EstablishedState) -> MessageFrame {
        if self.next_chunk == -1 {
            return self.send_o_header(session);
        } else {
            return self.send_o_chunk(session);
        }
    }
}

//////////////////////////
// ObjectReceiveJob

/// Will be called by the transport layer to pass a chunk on to
/// the application. There is one ChunkListener per Object.
/// Number of application tlvs in this chunk in `nr_tlv`.
pub type ChunkListener = dyn FnMut(Vec<u8>, ChunkId, u8) -> ();

pub struct ObjectReceiveJob {
    pub chunk_received_callback: Box<ChunkListener>,
    /// Metadata about the object.
    pub object: Object,
    /// Abort receiving by setting this flag to true.
    pub abort: bool,
}
