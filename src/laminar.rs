use laminar::Socket;
use laminar::{self, Packet};
use std::net::SocketAddr;

use crate::SynchronizeOutbound;

impl SynchronizeOutbound<SocketAddr> for Socket {
    fn synchronize(&mut self, bound: SocketAddr, data: Vec<u8>) {
        self.send(Packet::reliable_ordered(bound, data.clone(), None))
            .unwrap();
    }
}
