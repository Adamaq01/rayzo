use laminar::Config;
use laminar::Packet;
use laminar::Socket;
use laminar::SocketEvent;
use rayzo::resources::Resources;
use rayzo::server::Server;
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize, SerdeDiff)]
struct Count(usize);

fn main() {
    let updates_per_second = 1;

    println!("Starting counting server...");

    let config = Config {
        heartbeat_interval: Some(Duration::from_millis(1000)),
        ..Config::default()
    };

    let server = Arc::new(Mutex::new(Server::new(
        Socket::bind_with_config("127.0.0.1:12346", config).unwrap(),
    )));
    let event_receiver = server.lock().unwrap().get_event_receiver().clone();
    server
        .lock()
        .unwrap()
        .resources_mut()
        .register_outbound("count".into(), Count(0));

    let server_2 = Arc::clone(&server);
    let _thread = thread::spawn(move || loop {
        if let Ok(event) = event_receiver.recv() {
            let mut server = server_2.lock().unwrap();
            match event {
                SocketEvent::Packet(packet) => {
                    if server.is_connected(packet.addr()) {
                        server.synchronize_inbound(packet.addr(), packet.payload().to_vec());
                    } else {
                        server
                            .send(Packet::reliable_unordered(packet.addr(), b"".to_vec()))
                            .unwrap();
                    }
                }
                SocketEvent::Connect(address) => {
                    server.register_connection(address);
                }
                SocketEvent::Timeout(address) => {
                    server.remove_connection(address);
                }
                _ => {}
            }
        }
    });

    let mut should_exit = false;
    let mut previous_instant = Instant::now();
    let fixed_time_step = 1.0 / updates_per_second as f64;
    while !should_exit {
        let current_instant = Instant::now();
        let elapsed = current_instant
            .duration_since(previous_instant)
            .as_secs_f64();
        if elapsed >= fixed_time_step {
            let mut server = server.lock().unwrap();
            should_exit = update(&mut server);

            server.synchronize_outbound();
            server.manual_poll(Instant::now());

            previous_instant = current_instant;
        }
    }
}

fn update(server: &mut Server<Socket, SocketAddr>) -> bool {
    let res = server
        .resources_mut()
        .outbound_mut::<Count>("count".into())
        .unwrap();
    res.0 += 1;
    false
}
