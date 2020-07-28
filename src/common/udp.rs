use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::net::UdpSocket;
use std::time::Duration;

const BUFFERSIZE: u32 = 10; //TODO

pub struct Socket {}

impl Socket {
 
    pub fn new_server() -> Socket {
        let addr = "0.0.0.0:12057";
        let mut socket: UdpSocket = UdpSocket::bind(addr).expect("Unable to bind to address"); //TODO make configurable

        spawn_rx_thread(&socket);

        let mut buf = [0; 10];
        for i in 1..10 {
            println!("recv");
            let (nr_of_bytes, src) = socket.recv_from(&mut buf).unwrap();
            socket.send_to(&buf, &src);
            thread::sleep(Duration::from_secs(1));
        }

        return Socket{};
    }

    pub fn try_recv() {
        
    }
}

/// receives on rx (network) and sends to tx (other thread)
fn rx_thread(mut rx: &UdpSocket, tx: Sender<[u8; 10]>) {
    for i in 1..5 {
        println!("rx thread: recv");
        let mut buf = [0; 10];
        let (nr_of_bytes, src) = rx.recv_from(&mut buf).unwrap();
        tx.send(buf).unwrap();
        thread::sleep(Duration::from_secs(2));
    }
}


fn spawn_rx_thread(mut socket: &'static UdpSocket) -> Receiver<[u8; 10]> {
    let (tx, rx) = channel::<[u8; 10]>();
    let joinable = thread::spawn(move || rx_thread(socket, tx));
    for i in 1..10 {
        print!("Got: ");
        if let Ok(vec) = rx.try_recv() {
            print!("{:?}", vec);
        }
        print!("\n");
        thread::sleep(Duration::from_secs(1));
    }
    return rx;
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
    spawn_rx_thread()
}
