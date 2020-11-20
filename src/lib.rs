pub mod client;
pub mod node;
pub mod resources;
pub mod server;

#[cfg(feature = "laminar")]
pub mod laminar;

use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Target<I> {
    Specific(I),
    All,
}

pub trait SynchronizeOutbound<B> {
    fn synchronize(&mut self, bound: B, data: Vec<u8>);
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ResourcePacket {
    identifier: String,
    data: Vec<u8>,
}
