use crate::transport::jobs::*;
use std::net::{SocketAddr, UdpSocket};
use crate::common::udp::{Socket, Packet};
use std::rc::Rc;
use super::frame::*;
use crate::common::{Cursor, WireFormat, ReadResult};
use log;
use rand::{thread_rng, Rng};
use crate::transport::common::*;
use std::iter::Iterator;

//////////////////////////
// Connection

/// Will be called by the transport layer to notify the application
/// about new receiving Objects.
/// The application returns None, if it is not interested in the Object.
/// Otherwise the application returns its ChunkListener for that Object.
pub type ObjectListener = dyn FnMut (ObjectReceiveJob) -> ();

/// Will be called by the transport layer to inform the application
/// about a timeout of a connection.
pub type TimeoutListener = dyn FnMut () -> ();

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

impl Connection{
    /// Should return within about 0.1s to allow the application to interact
    /// with the user still.
    /// Must be called by the application in its main loop.
    pub fn receive_and_send(&mut self)
    {
        for i in 0..self.send_jobs.len() {
            self.send_once(i);
        }

        self.receive_once();
    }

    fn send_once(&mut self, i: usize) {
        //let mut job: ObjectSendJob = self.send_jobs.remove(i);
        // get session or return
        let mut session: &EstablishedState;
        if let Some(s) = &self.session {
            session = s;
        } else {
            log::warn!("Connection.send_once(): Session not yet established.");
            return;
        }

        self.send_jobs[i].send_step();
    }

    /// receive the next packet
    /// non-blocking
    fn receive_once(&mut self) {
        // try to receive a packet
        let mut buf: Vec<u8> = Vec::new();
        let mut message_frame = MessageFrame::default();
        if let Ok(n_bytes) = self.socket.recv(&mut buf) {
            let mut cursor = Cursor::new(buf[0..n_bytes].to_vec());
            match message_frame.read(&mut cursor) {
                ReadResult::Err(x) => {
                    log::error!("Error: {}", &x.to_string());
                    return;
                },
                _ => {},
            }
        }

        // check protocol version
        if message_frame.version != PROTOCOL_VERSION {
            unimplemented!("Protocol doesnt match.");
        }

        // get first tlv
        if message_frame.tlvs.len() <= 1 {
            log::warn!("Received MessageFrame without tlvs.");
            return;
        }
        let tlv: Tlv = message_frame.tlvs.remove(0);

        // check if handshake is done
        if self.peer_info.is_none() {
            // we are waiting for peer info
            if let Tlv::HostInformation(i) = tlv {
                // save peer info and complete handshake
                self.peer_info = Some(i);
                if self.is_server {
                    self.send_handshake();
                }
            } else {
                log::debug!("This is not the HostInformation tlv we are waiting for. It must be the first TLV in a message.");
                return;
            }
        }
        // if is_server: we have received and send HostInfos.
        // if !is_server: we have sent and received HostInfos.
        // => this is an ordinary, established connection now.
        // Therefore we may unwrap:
        //let peer_info: HostInformation = self.peer_info.unwrap();
        //let session: EstablishedState = self.session.unwrap();

        log::info!("Session (id: {}) established.", self.session.as_ref().unwrap().sessionid);
        todo!();
    }

    /// must be called before anything is sent.
    /// After calling this, self.session will be Some().
    pub(super) fn send_handshake(&mut self) {
        // create message frame
        let mut frame: MessageFrame = MessageFrame::default();
        frame.version = PROTOCOL_VERSION;
        if self.is_server {
            // set random session id
            frame.sid = thread_rng().gen();
        } else {
            // we are a client
            frame.sid = 0;
        }
        frame.tlvs.insert(0, Tlv::HostInformation(self.self_info.clone()));

        // serialize and send frame
        let mut cursor = Cursor::new(Vec::new());
        frame.write(&mut cursor);
        self.socket.send(&(cursor.into_inner()));

        // now we can carefully initialize the session
        self.session = Some(EstablishedState::be_gentle(frame.sid));
    }
}

pub struct EstablishedState {
    sessionid: SessionId,
}

impl EstablishedState {
    /// returns a state that ensures the connection will be as gentle as
    /// possible to its peer.
    fn be_gentle(sessionid: SessionId) -> Self {
        Self {
            sessionid,
        }
    }
}
