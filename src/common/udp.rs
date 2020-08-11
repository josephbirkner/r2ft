use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};
use std::thread;
use std::net::{UdpSocket, SocketAddr};
use crate::common::mtu;

type Buffer = [u8; 16];

/// UdpPacket with a buffer filled up to usize sent by SocketAddr.
#[derive(Debug)]
pub struct Packet (Buffer, usize, SocketAddr);

impl PartialEq for Packet {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1 && self.2 == other.2
    }
}

/// Starts a thread to offer non-blocking `send` and `try_recv` for UDP.
pub struct Socket {
    terminate_thread: Sender<bool>,
    receiver: Receiver<Packet>,
    sender: UdpSocket,
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.terminate_thread.send(true).unwrap();
    }
}

impl Socket {
 
    pub fn bind(addr: SocketAddr) -> Socket {
        let read_socket: UdpSocket = UdpSocket::bind(addr).expect("Unable to bind to address");
        let write_socket: UdpSocket = read_socket.try_clone().expect("Unable to clone socket");

        let (receiver, send_terminate) = Socket::spawn_rx_thread(read_socket);

        return Socket{ 
            terminate_thread: send_terminate,
            receiver: receiver,
            sender: write_socket,
        };
    }

    /// Receive non-blockingly. 
    pub fn try_recv(&self) -> Option<Packet> {
        match self.receiver.try_recv() {
            Ok(m) => Some(m),
            Err(TryRecvError::Disconnected) => panic!("Receiver thread dead"),
            Err(TryRecvError::Empty) => None
        }
    }

    /// Send payload to addr (may block).
    pub fn send(&self, payload: Buffer, addr: SocketAddr) {
        let _n_bytes_sent = self.sender.send_to(&payload, addr).expect("IP version of Socket and addr do not match.");
    }

    /// receives on rx (network) and sends to tx (other thread)
    fn rx_thread(rx: UdpSocket, tx: Sender<Packet>, terminate: Receiver<bool>) {
        loop {
            let mut buf = [0; 16];
            let (nr_of_bytes, src) = rx.recv_from(&mut buf).unwrap();
            tx.send(Packet(buf, nr_of_bytes, src)).unwrap();
            match terminate.try_recv() {
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => return,
                Ok(_) => return,
            }
        }
    }

    /// spawns a receiving thread which terminates as soon as receiving anything on `terminate`
    fn spawn_rx_thread(socket: UdpSocket) -> (Receiver<Packet>, Sender<bool>) {
        // channel to terminate thread
        let (send_terminate, receive_terminate) = channel::<bool>();

        // channel to communicate received packets
        let (tx, rx) = channel::<Packet>();

        let _joinable = thread::spawn(move || Socket::rx_thread(socket, tx, receive_terminate));
        return (rx, send_terminate);
    }
}


mod test {
    #[test]
    fn test_threads() {
        use super::{Socket, Buffer, Packet};

        let a_addr = "0.0.0.0:12057".parse().unwrap();
        let b_addr = "0.0.0.0:3333".parse().unwrap();
        let src = "127.0.0.1:12057".parse().unwrap();
        let dest = "127.0.0.1:3333".parse().unwrap();
        let a = Socket::bind(a_addr);
        let b = Socket::bind(b_addr);

        // receive nothing and return immediately
        assert_eq!(b.try_recv(), None);

        // send and expect to receive after giving OS time for delivery
        let sent: Buffer = [42; 16];
        a.send(sent, dest);
        std::thread::sleep(std::time::Duration::from_secs_f32(0.1));
        assert_eq!(b.try_recv(), Some(Packet(sent, 16 as usize, src)));
        
        // receive nothing and return immediately
        assert_eq!(b.try_recv(), None);
    }
}
