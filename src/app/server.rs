use super::state::*;
use crate::options::Options;
use crate::transport::server;
use crate::transport::server::Listener;
use log::info;
use std::cell::RefCell;
use std::env::current_dir;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::rc::Rc;

/// Run server on current working directory
pub fn run(opt: Options) -> std::result::Result<(), ()> {
    //////////////////////////////
    // Server client startup.
    let mut s = format!(
        "File server started with {}, working directory {}",
        opt,
        current_dir().unwrap().display()
    );

    //////////////////////////////
    // Create shared state machine.
    let state_machine = Rc::new(RefCell::new(StateMachine::new()));

    //////////////////////////////
    // Create server for listening
    let mut server = Listener::new(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(0, 0, 0, 0),
        opt.port,
    )));

    //////////////////////////////
    // Wait until reception is done.
    while !state_machine.borrow().is_finished() {
        ///////////////////////////////////
        // Create potential event handlers.
        let incoming_object_handler = Box::new(move |recv_job| {});
        let state_machine_for_timeout_handler = Rc::clone(&state_machine);
        let timeout_handler = Box::new(move || {
            state_machine_for_timeout_handler.borrow_mut().finished();
        });

        ///////////////////////////////////
        // Listen for connection
        let mut connection = match server.listen_once(incoming_object_handler, timeout_handler) {
            Some(connection) => connection,
            None => continue,
        };

        while !state_machine.borrow().is_finished() {
            connection.receive_and_send();

            ///////////////////////////////////
            // Register new receive jobs
            for recv_job in &mut connection.recv_jobs {
                if !state_machine.borrow().has_recv_job(recv_job) {
                    StateMachine::push_recv_job(&state_machine, recv_job);
                }
            }

            ///////////////////////////////////
            // Push out new send jobs
            loop {
                match state_machine.borrow_mut().pop_new_send_job() {
                    Some(job) => connection.send_jobs.push(job),
                    None => break,
                }
            }
        }
    }

    Ok(())
}
