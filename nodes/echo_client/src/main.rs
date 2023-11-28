use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use libproto::echo::{Echo, EchoOk};
use libproto::Message;
use libproto::system::{BeginMonitor, MonitorEvent, MonitorEventKind, Setup, SetupOk};

static NUM_ECHOES: usize = 10;
static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

fn main() -> Result<(), Box<dyn Error>> {
    let receiver = receiver();

    for num_clients in 1..=2 {
        for num_servers in 1..=2 {
            eprintln!("testing with {num_clients} clients and {num_servers} servers");
            if !test(&receiver, num_servers, num_clients)? {
                eprintln!("test failed");
                return Ok(());
            }
        }
    }
    eprintln!("All tests OK");
    Ok(())
}

fn test(receiver: &Receiver<Message>, num_servers: usize, num_clients: usize) -> Result<bool, Box<dyn Error>> {
    let mut clients = HashMap::<String, (Sender<Message>, JoinHandle<bool>)>::new();

    let client_names: Vec<String> = (0..num_clients).map(|id| format!("c{id}")).collect();
    let server_names: Vec<String> = (0..num_servers).map(|id| format!("s{id}")).collect();

    Message::new("client", "core", None, Setup {
        clients: client_names,
        servers: server_names.clone(),
    }).send();
    receiver.recv().unwrap().payload::<SetupOk>().unwrap();
    eprintln!("setup ok!");

    for i in 0..num_clients {
        let (tx, rx) = channel();
        let name = format!("c{i}");
        let servers = server_names.clone();
        clients.insert(name.clone(), (tx, thread::spawn(move || single_client(name, servers, rx))));
    }

    while clients.is_empty() || clients.iter().any(|(_, (_, c))| !c.is_finished()) {
        let msg = match receiver.recv_timeout(Duration::from_millis(250)) {
            Ok(msg) => msg,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        clients.get(&msg.dst)
            .unwrap()
            .0.send(msg)
            .ok();
    }

    let mut ok = true;
    for (_, (_, thread)) in clients {
        ok &= thread.join().unwrap()
    }

    Ok(ok)
}

fn single_client(name: String, servers: Vec<String>, receiver: Receiver<Message>) -> bool {
    eprintln!("sending begin monitor message");
    Message::new(&name, "core", None, BeginMonitor { src_match: "s*".to_owned(), dst_match: name.clone() })
        .send();

    for i in 0..NUM_ECHOES {
        'echoing: for server_name in &servers {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            send_echo(&name, server_name, id, format!("echo from {name} to {server_name}: {i}"));
            loop {
                let Ok(message) = receiver.recv() else {
                    eprintln!("not all echoes were replied to");
                    return false;
                };
                if let Ok(event) = message.payload::<MonitorEvent>() {
                    match event.kind {
                        MonitorEventKind::Sent =>
                            eprintln!("[{name}]MONITOR: sent message at ({}, {}): `{}`",
                                      event.timestamp_logical,
                                      event.timestamp_physical,
                                      event.message),
                        MonitorEventKind::Delivered =>
                            eprintln!("[{name}]MONITOR: delivered message {} at ({}, {}): `{}`",
                                      event.in_reference_to.unwrap(),
                                      event.timestamp_logical,
                                      event.timestamp_physical,
                                      event.message),
                    };
                } else {
                    if !check_response(message, id, &name, server_name) {
                        return false;
                    } else {
                        continue 'echoing;
                    };
                }
            }
        }
    }
    true
}

fn check_response(message: Message, id: usize, src: &str, dst: &str) -> bool {
    let response = match message.payload::<EchoOk>() {
        Ok(r) => r,
        Err(_) => {
            eprintln!("invalid response type");
            return false;
        }
    };

    let msg_id = match message.body.in_reply_to {
        Some(id) => id,
        None => {
            eprintln!("response without in_reply_to field");
            return false;
        }
    };

    if msg_id != id {
        eprintln!("received echo reply with wrong in_reply_to id");
        return false;
    }

    if message.src != dst || message.dst != src {
        eprintln!("received echo reply from wrong node");
        return false;
    }

    if response.echo != message.body.data["echo"].as_str().unwrap() {
        eprintln!("received echo reply with wrong payload");
        return false;
    }

    true
}

fn send_echo(src: &str, dst: &str, msg_id: usize, echo: String) {
    Message::new(src, dst, Some(msg_id), Echo { echo }).send();
}

fn receiver() -> Receiver<Message> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        for msg in Message::recv_iter().map(Result::unwrap) {
            if tx.send(msg).is_err() {
                return;
            }
        }
    });
    rx
}