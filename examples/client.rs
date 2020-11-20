use laminar::Config;
use laminar::Packet;
use laminar::Socket;
use laminar::SocketEvent;
use rayzo::client::Client;
use rayzo::resources::Resources;
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
    let updates_per_second = 5;

    println!("Starting client");

    let config = Config {
        heartbeat_interval: Some(Duration::from_millis(1000)),
        ..Config::default()
    };

    let client = Arc::new(Mutex::new(Client::<Socket, SocketAddr>::new(
        Socket::bind_with_config("127.0.0.1:12345", config).unwrap(),
        "127.0.0.1:12346".parse().unwrap(),
    )));
    let event_receiver = client.lock().unwrap().get_event_receiver().clone();
    client
        .lock()
        .unwrap()
        .resources_mut()
        .register_inbound("count".into(), Count(0));

    let client_2 = Arc::clone(&client);
    let _thread = thread::spawn(move || loop {
        if let Ok(event) = event_receiver.recv() {
            let mut client = client_2.lock().unwrap();
            match event {
                SocketEvent::Packet(packet) => {
                    client.synchronize_inbound(packet.payload().to_vec());
                }
                _ => {}
            }
        }
    });

    client
        .lock()
        .unwrap()
        .send(Packet::reliable_unordered(
            "127.0.0.1:12346".parse().unwrap(),
            b"".to_vec(),
        ))
        .unwrap();

    let mut should_exit = false;
    let mut previous_instant = Instant::now();
    let fixed_time_step = 1.0 / updates_per_second as f64;
    while !should_exit {
        let current_instant = Instant::now();
        let elapsed = current_instant
            .duration_since(previous_instant)
            .as_secs_f64();
        if elapsed >= fixed_time_step {
            let mut client = client.lock().unwrap();
            should_exit = update(&mut client);

            client.synchronize_outbound();
            client.manual_poll(Instant::now());

            previous_instant = current_instant;
        }
    }
}

fn update(client: &mut Client<Socket, SocketAddr>) -> bool {
    let res = client
        .resources()
        .inbound::<Count>("count".into())
        .unwrap()
        .0;
    println!("Value: {}", res);
    false
}
