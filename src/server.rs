use crate::resources::HashMapResources;
use crate::{node::Node, ResourcePacket, SynchronizeOutbound, Target};
use std::{
    collections::HashMap,
    hash::Hash,
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Serialize};

pub struct Server<S, C>
where
    S: SynchronizeOutbound<C>,
    C: Eq + Hash + Clone,
{
    pub(crate) connections: HashMap<C, HashMap<OutboundIdentifier<C>, usize>>,
    pub(crate) node: Node<
        S,
        InboundIdentifier<C>,
        OutboundIdentifier<C>,
        HashMapResources<InboundIdentifier<C>, OutboundIdentifier<C>>,
    >,
}

impl<S, C> Deref for Server<S, C>
where
    S: SynchronizeOutbound<C>,
    C: Eq + Hash + Clone,
{
    type Target = Node<
        S,
        InboundIdentifier<C>,
        OutboundIdentifier<C>,
        HashMapResources<InboundIdentifier<C>, OutboundIdentifier<C>>,
    >;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<S, C> DerefMut for Server<S, C>
where
    S: SynchronizeOutbound<C>,
    C: Eq + Hash + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}

impl<S, C> Server<S, C>
where
    S: SynchronizeOutbound<C>,
    C: Eq + Hash + Clone,
{
    pub fn new(node: S) -> Self {
        Self {
            connections: HashMap::new(),
            node: Node::new(node),
        }
    }

    pub fn register_connection(&mut self, connection: C) {
        self.connections.insert(connection, HashMap::new());
    }

    pub fn remove_connection(&mut self, connection: C) {
        self.connections.remove(&connection);
    }

    pub fn is_connected(&self, connection: C) -> bool {
        self.connections.contains_key(&connection)
    }

    pub fn synchronize_outbound_fully(&mut self, identifier: OutboundIdentifier<C>) {
        match &identifier.1 {
            Target::Specific(connection) => {
                if let Some(generations) = self.connections.get_mut(&connection) {
                    generations.remove(&identifier);
                }
            }
            Target::All => {
                for generations in self.connections.values_mut() {
                    generations.remove(&identifier);
                }
            }
        }
    }

    pub fn synchronize_outbound(&mut self) {
        let resources = &mut self.node.resources.outbound_resources;
        let connections = &mut self.connections;
        let node = &mut self.node.node;

        for (identifier, resource) in resources.iter_mut() {
            if resource.is_dirty() {
                match &identifier.1 {
                    Target::Specific(connection) => {
                        let generations = connections.get_mut(&connection).unwrap();
                        let generation = generations.get_mut(&identifier);
                        let packet = ResourcePacket {
                            identifier: identifier.0.clone(),
                            data: resource.serialize(generation.is_none()).unwrap(),
                        };
                        let payload = bincode::serialize(&packet).unwrap();
                        node.synchronize(connection.clone(), payload.clone());
                    }
                    Target::All => {
                        let full_packet = ResourcePacket {
                            identifier: identifier.0.clone(),
                            data: resource.serialize(true).unwrap(),
                        };
                        let full_payload = bincode::serialize(&full_packet).unwrap();
                        let diff_packet = ResourcePacket {
                            identifier: identifier.0.clone(),
                            data: resource.serialize(false).unwrap(),
                        };
                        let diff_payload = bincode::serialize(&diff_packet).unwrap();
                        for (connection, generations) in connections.iter_mut() {
                            let generation = generations.get_mut(&identifier);
                            let payload = if generation.is_none() {
                                &full_payload
                            } else {
                                &diff_payload
                            };
                            node.synchronize(connection.clone(), payload.clone());
                        }
                    }
                }
                resource.snapshot();
                match &identifier.1 {
                    Target::Specific(connection) => {
                        let generations = connections.get_mut(&connection).unwrap();
                        generations.insert(identifier.clone(), resource.generation().unwrap());
                    }
                    Target::All => {
                        for generations in connections.values_mut() {
                            generations.insert(identifier.clone(), resource.generation().unwrap());
                        }
                    }
                }
                resource.set_dirty(false);
            }
        }
    }

    pub fn synchronize_inbound(&mut self, connection: C, data: Vec<u8>) {
        let packet = bincode::deserialize::<ResourcePacket>(&data);
        if let Ok(packet) = packet {
            let identifier = InboundIdentifier(packet.identifier, connection);
            self.node
                .resources_mut()
                .inbound_resources
                .get_mut(&identifier)
                .unwrap()
                .deserialize(&packet.data);
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InboundIdentifier<I>(pub String, pub I);

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutboundIdentifier<I>(pub String, pub Target<I>);

impl<I> From<String> for OutboundIdentifier<I> {
    fn from(s: String) -> Self {
        Self(s, Target::All)
    }
}

impl<I> From<&str> for OutboundIdentifier<I> {
    fn from(s: &str) -> Self {
        Self(s.into(), Target::All)
    }
}
