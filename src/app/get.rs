use super::state::*;
use crate::options::Options;
use crate::transport::client;
use log::info;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::{thread, time};

/// Run client for file retrieval.
pub fn get(opt: Options, socket_addr: SocketAddr, files: Vec<&str>) -> std::result::Result<(), ()> {
    //////////////////////////////
    // Announce client startup.
    let mut s = format!(
        "File client started with {} for socket address {} and file(s) '",
        opt, socket_addr
    );
    for f in &files {
        s = format!("{} {}", s, f);
    }
    info!("{} '", s);

    //////////////////////////////
    // Create shared state machine.
    let state_machine = Rc::new(RefCell::new(StateMachine::new()));
    let state_machine_for_timeout_handler = Rc::clone(&state_machine);

    //////////////////////////////
    // Create event handlers to be used as callbacks.
    // Right now they have no purpose for the server, so just ignore.
    let incoming_object_handler = Box::new(move |recv_job| {}); // unused, does nothing
    let timeout_handler = Box::new(move || {
        state_machine_for_timeout_handler.borrow_mut().finished();
    });

    //////////////////////////////
    // Create connection.
    let mut connection = client::connect(socket_addr, incoming_object_handler, timeout_handler);
    state_machine.borrow_mut().connected(); // does not have an impact on anything

    //////////////////////////////
    // Request files by adding the corresponding send jobs to our client state(machine)
    let mut files_copy = Vec::new();
    for path in files {
        files_copy.push(String::from(path));
    }
    //////////////////////////////
    // Add initial jobs
    connection
        .send_jobs
        .push(state_machine.borrow_mut().push_file_request_job(files_copy));

    //////////////////////////////
    // Wait until reception is done.
    // State of statemachine is used since it may also be changed via callbacks
    while !state_machine.borrow().is_finished() {
        connection.receive_and_send();

        ///////////////////////////////////
        // Register new receive jobs from connection to state
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

        ///////////////////////////////////
        // Stop loop if files were received
        if state_machine.borrow().all_files_received() {
            state_machine.borrow_mut().finished();
        }

        thread::sleep(time::Duration::from_millis(1));
    }

    Ok(())
}
