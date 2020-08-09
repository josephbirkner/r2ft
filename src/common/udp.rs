use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};
use std::thread;
use std::thread::JoinHandle;
use std::net::UdpSocket;
use std::time::Duration;

const BUFFERSIZE: u32 = 10; //TODO

pub struct Socket {
    joinable: JoinHandle<()>,
}

impl Drop for Socket {
    fn drop(&mut self) {
        println!("dropping and joining");
        //self.joinable.join();
        println!("dropped and joined");
    }
}

impl Socket {
 
    pub fn new_server() -> Socket {
        let addr = "0.0.0.0:12057";
        let mut read_socket: UdpSocket = UdpSocket::bind(addr).expect("Unable to bind to address"); //TODO make configurable
        let mut write_socket: UdpSocket = read_socket.try_clone().expect("Unable to clone socket");
        let (tx, rx) = channel::<bool>();
        let (_, joinable) = Socket::spawn_rx_thread(read_socket, rx);

        let ret = Socket{ joinable: joinable, };
//        let mut buf = [0; 10];
//        for i in 1..10 {
//            println!("recv");
//            let (nr_of_bytes, src) = read_socket.recv_from(&mut buf).unwrap();
//            read_socket.send_to(&buf, &src);
//            thread::sleep(Duration::from_secs(1));
//        }

        return ret;
    }

    pub fn try_recv() {
        
    }

    /// receives on rx (network) and sends to tx (other thread)
    fn rx_thread(mut rx: UdpSocket, tx: Sender<[u8; 10]>, terminate: Receiver<bool>) {
        for i in 1..5 {
            println!("rx thread: recv");
            let mut buf = [0; 10];
            let (nr_of_bytes, src) = rx.recv_from(&mut buf).unwrap();
            tx.send(buf).unwrap();
            match terminate.try_recv() {
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => return,
                Ok(_) => return,
            }
            thread::sleep(Duration::from_secs(2));
        }
    }

    fn spawn_rx_thread(mut socket: UdpSocket, terminate: Receiver<bool>) -> (Receiver<[u8; 10]>, JoinHandle<()>) {
        let (tx, rx) = channel::<[u8; 10]>();
        let joinable = thread::spawn(move || Socket::rx_thread(socket, tx, terminate));
        for i in 1..10 {
            print!("Got: ");
            if let Ok(vec) = rx.try_recv() {
                print!("{:?}", vec);
            }
            print!("\n");
            thread::sleep(Duration::from_secs(1));
        }
        return (rx, joinable);
    }
}




#[test]
fn send() {
    let addr = "0.0.0.0:3333"; //TODO client src port random
    let mut socket = UdpSocket::bind(addr).unwrap();
    socket.connect("0.0.0.0:12057").unwrap();
    socket.send(&[0,1,2]).unwrap();
}

#[test]
fn test_threads() {
    let a = Socket::new_server();
}
