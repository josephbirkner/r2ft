use super::frame::*;
use crate::common::{Cursor, ReadResult, WireFormat};
use crate::transport::common::*;
use crate::transport::jobs::*;
use std::ops::FnMut;
use log;
use rand::{thread_rng, Rng};
use std::net::{SocketAddr, UdpSocket};
use std::ops::DerefMut;

//////////////////////////
// Connection

/// Will be called by the transport layer to notify the application
/// about new receiving Objects.
/// The application returns None, if it is not interested in the Object.
/// Otherwise the application returns its ChunkListener for that Object.
pub type ObjectListener = dyn FnMut(ObjectReceiveJob) -> ();

/// Will be called by the transport layer to inform the application
/// about a timeout of a connection.
pub type TimeoutListener = dyn FnMut() -> ();

/// Constructors for `Connection` are found in `super::{client, server}`.
pub struct Connection {
    pub send_jobs: Vec<ObjectSendJob>,
    pub recv_jobs: Vec<ObjectReceiveJob>,

    /// TODO remove
    pub(super) accept_callback: Box<ObjectListener>,
    pub(super) timeout_callback: Box<TimeoutListener>,
    pub(super) socket: UdpSocket,
    /// Target of communication to send to.
    dest: SocketAddr,
    pub(super) is_server: bool,
    pub(super) self_info: HostInformation,

    /// ## Handshake Procedure
    ///
    /// 1. peer_info and session are None
    /// 2. If you are a server, peer_info is Some() on receiving a clients handshake. Afterwards
    ///    you send your handshake.
    /// 3. If you are a client, you send your handshake first.
    /// 2. As soon as your handshake is sent, your session is Some().
    /// 3. Now you have an ordinary, established connection.
    pub(super) peer_info: Option<HostInformation>,
    pub(super) session: Option<EstablishedState>,
}

impl Connection {
    /// Should return within about 0.1s to allow the application to interact
    /// with the user still.
    /// Must be called by the application in its main loop.
    pub fn receive_and_send(&mut self) {
        for i in 0..self.send_jobs.len() {
            self.send_once(i);
        }
        self.receive_once();
    }

    fn send_once(&mut self, i: usize) {
        //let mut job: ObjectSendJob = self.send_jobs.remove(i);
        // only send, if state is established
        if self.session.is_none() {
            log::warn!("Connection.send_once(): Session not yet established.");
            return;
        }

        let session = self.session.as_ref().unwrap();
        let msg = self.send_jobs[i].send_next(&session);

        // serialize and send frame
        if let Some(msg) = &msg {
            let mut cursor = Cursor::new(Vec::new());
            msg.write(&mut cursor);
            let buf = cursor.into_inner();
            let n_sent = self.socket.send(&buf).unwrap();
            assert_eq!(n_sent, buf.len());
            self.send_jobs[i].next_chunk += 1;
        }
    }

    /// receive the next packet
    /// non-blocking
    fn receive_once(&mut self) {
        // try to receive a packet
        let mut buf: [u8; MAX_UDP_BUFSIZE] = [0; MAX_UDP_BUFSIZE];
        let mut message_frame = MessageFrame::default();
        if let Ok(n_bytes) = self.socket.recv(&mut buf) {
            let mut cursor = Cursor::new(buf[0..n_bytes].to_vec());
            match message_frame.read(&mut cursor) {
                ReadResult::Err(x) => {
                    log::error!("MessageFrame read error: {}", &x.to_string());
                    return;
                }
                _ => {}
            }
        } else {
            // no packets received
            return;
        }
        log::trace!(
            "Received: proto version {}, sid {}, n_tlvs {}",
            message_frame.version,
            message_frame.sid,
            message_frame.tlvs.len()
        );

        // check protocol version
        if message_frame.version != PROTOCOL_VERSION {
            unimplemented!("Protocol version doesn't match.");
        }

        if message_frame.tlvs.len() <= 0 {
            log::warn!("Received MessageFrame without tlvs.");
            return;
        }

        for tlv in &message_frame.tlvs {
            self.accept_tlv(&message_frame, tlv);
        }
    }

    fn accept_tlv(&mut self, frame: &MessageFrame, tlv: &Tlv)
    {
        match (&self.peer_info, tlv) {
            // we are waiting for peer info
            (None, Tlv::HostInformation(hi)) => {
                // save peer info and complete handshake
                self.peer_info = Some(hi.clone());
                if self.is_server {
                    self.send_handshake();
                } else {
                    self.session = Some(EstablishedState::be_gentle(frame.sid));
                }
                // if is_server: we have received and send HostInfos.
                // if !is_server: we have sent and received HostInfos.
                // => this is an ordinary, established connection now.
                // Therefore we may unwrap:
                //let peer_info: HostInformation = self.peer_info.unwrap();
                //let session: EstablishedState = self.session.unwrap();
                log::info!(
                    "Session (id: {}) established.",
                    self.session.as_ref().unwrap().sessionid
                );
            }
            (None, _) => {
                log::debug!("This is not the HostInformation tlv we are waiting for. It must be the first TLV in a message.");
                return;
            }
            (_, Tlv::ObjectHeader(oh)) => {
                // todo!("Match peer info to correct connection.");
                self.recv_jobs.push(ObjectReceiveJob {
                    chunk_received_callback: Box::new(|_, _, _| {}),
                    object: Object {
                        object_type: oh.object_type,
                        object_id: oh.object_id,
                        fields: Clone::clone(&oh.fields),
                        transmission_finished_callback: Box::new(|| {}),
                    },
                    abort: false,
                    ack_req: if oh.ack_req { -1 } else { -2 },
                });
            }
            (_, Tlv::ObjectChunk(oc)) => {
                // todo!("Match peer info to correct connection.");
                match self.recv_jobs.iter_mut().find(|job|{
                    job.object.object_id == oc.object_id
                }) {
                    Some(recv_job) => {
                        if oc.ack_required { recv_job.ack_req = oc.chunk_id }
                        let fun = &mut recv_job.chunk_received_callback;
                        fun(oc.data.clone(), oc.chunk_id, oc.num_enclosed_msgs);
                    },
                    None =>
                        log::warn!("Received chunk for object {} with no active receive job.",
                            oc.object_id)
                };
            },
            (_, Tlv::ObjectAckRequest(ar)) => {
                // for every ackreq ...
                for (objectid, chunkid) in &ar.req_ack_object_chunks {
                    // ... find the job for the object ...
                    match self.recv_jobs.iter_mut().find(|job|{
                        job.object.object_id == *objectid
                    }) {
                        Some(recv_job) => {
                            // ... and set the req ack
                            recv_job.ack_req = *chunkid;
                        },
                        None => log::warn!("Received AckReq for unknown Object.")
                    }
                }
            },
            (_, _) => unimplemented!(),
        }
    }

    /// must be called before anything is sent.
    /// After calling this, self.session will be Some().
    pub(super) fn send_handshake(&mut self) {
        // create message frame
        let mut frame: MessageFrame = MessageFrame::default();
        frame.version = PROTOCOL_VERSION;
        if self.is_server {
            // set random session id
            frame.sid = thread_rng().gen_range(1, 2 ^ 64);
        } else {
            // we are a client
            frame.sid = 0;
        }
        frame
            .tlvs
            .insert(0, Tlv::HostInformation(self.self_info.clone()));

        // serialize and send frame
        let mut cursor = Cursor::new(Vec::new());
        frame.write(&mut cursor);
        let buf = cursor.into_inner();
        let n_sent = self.socket.send(&buf).unwrap();

        // now we can carefully initialize the session
        self.session = Some(EstablishedState::be_gentle(frame.sid));
    }
}

pub(super) struct EstablishedState {
    pub(super) sessionid: SessionId,
}

impl EstablishedState {
    /// returns a state that ensures the connection will be as gentle as
    /// possible to its peer.
    fn be_gentle(sessionid: SessionId) -> Self {
        Self { sessionid }
    }
}

mod test {
    #[test]
    fn handshake() {
        use env_logger;
        env_logger::init();
        use crate::transport::client;
        use crate::transport::connection::*;
        use crate::transport::server;
        use std::thread;
        use std::time::Duration;

        let mut connection_listener = server::Listener::new("0.0.0.0:8080".parse().unwrap());
        let mut server_conn: Option<Connection> =
            connection_listener.listen_once(Box::new(|a| {}), Box::new(|| {}));
        assert_eq!(server_conn.is_none(), true);

        let mut client_conn = client::connect(
            "127.0.0.1:8080".parse().unwrap(),
            Box::new(|a| {}),
            Box::new(|| {}),
        );
        thread::sleep(Duration::from_secs_f32(0.1));

        client_conn.receive_and_send();
        // initialized, but not complete yet
        assert_eq!(client_conn.session.as_ref().unwrap().sessionid, 0);

        server_conn = connection_listener.listen_once(Box::new(|a| {}), Box::new(|| {}));
        assert_eq!(server_conn.is_some(), true);

        client_conn.receive_and_send();
        assert_eq!(client_conn.session.as_ref().unwrap().sessionid, 0);

        let mut server_conn = server_conn.unwrap();
        server_conn.receive_and_send();
        // assert initialized
        assert_ne!(server_conn.session.unwrap().sessionid, 0);

        client_conn.receive_and_send();
        assert_ne!(client_conn.session.as_ref().unwrap().sessionid, 0);
    }
}
