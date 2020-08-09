use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};
use std::thread;
use std::thread::JoinHandle;
use std::net::{UdpSocket, SocketAddr};
use std::time::Duration;

const BUFFERSIZE: u32 = 10; //TODO

pub struct Socket {
    terminate_thread: Sender<bool>,
    receiver: Receiver<[u8; 10]>,
    sender: UdpSocket,
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.terminate_thread.send(true);
        println!("dropping and terminating");
    }
}

impl Socket {
 
    pub fn bind(addr: SocketAddr) -> Socket {
        let mut read_socket: UdpSocket = UdpSocket::bind(addr).expect("Unable to bind to address"); //TODO make configurable
        let mut write_socket: UdpSocket = read_socket.try_clone().expect("Unable to clone socket");
        let (tx, rx) = channel::<bool>();
        let receiver = Socket::spawn_rx_thread(read_socket, rx);

        let ret = Socket{ 
            terminate_thread: tx,
            receiver: receiver,
            sender: write_socket,
        };

        return ret;
    }

    pub fn try_recv(&self) -> Option<[u8; 10]> {
        match self.receiver.try_recv() {
            Ok(m) => Some(m),
            Err(TryRecvError::Disconnected) => panic!("Receiver thread dead"),
            Err(TryRecvError::Empty) => None
        }
    }

    pub fn send(&self, payload: [u8; 10], addr: SocketAddr) {
        self.sender.send_to(&payload, addr);
    }

    /// receives on rx (network) and sends to tx (other thread)
    fn rx_thread(mut rx: UdpSocket, tx: Sender<[u8; 10]>, terminate: Receiver<bool>) {
        loop {
            println!("rx thread: recv");
            let mut buf = [0; 10];
            let (nr_of_bytes, src) = rx.recv_from(&mut buf).unwrap();
            tx.send(buf).unwrap();
            match terminate.try_recv() {
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => return,
                Ok(_) => return,
            }
        }
    }

    /// spawns a receiving thread which terminates as soon as receiving anything on `terminate`
    fn spawn_rx_thread(mut socket: UdpSocket, terminate: Receiver<bool>) -> Receiver<[u8; 10]> {
        let (tx, rx) = channel::<[u8; 10]>();
        let joinable = thread::spawn(move || Socket::rx_thread(socket, tx, terminate));
        
        return rx;
    }
}


mod test {
    #[test]
    fn test_threads() {
        use super::Socket;

        let a_addr = "0.0.0.0:12057".parse().unwrap();
        let b_addr = "0.0.0.0:3333".parse().unwrap();
        let dest = "127.0.0.1:3333".parse().unwrap();
        let a = Socket::bind(a_addr);
        let b = Socket::bind(b_addr);

        // receive nothing and return immediately
        assert_eq!(b.try_recv(), None);

        // send and expect to receive after giving OS time for delivery
        let sent: [u8; 10] = [42; 10];
        a.send(sent, dest);
        std::thread::sleep(std::time::Duration::from_secs_f32(0.1));
        assert_eq!(b.try_recv(), Some(sent));
        
        // receive nothing and return immediately
        assert_eq!(b.try_recv(), None);
    }
}
