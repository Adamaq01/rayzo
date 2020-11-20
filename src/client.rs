use crate::node::Node;
use crate::resources::HashMapResources;
use crate::{ResourcePacket, SynchronizeOutbound};
use std::ops::Deref;
use std::ops::DerefMut;

#[allow(dead_code)]
pub struct Client<C, S>
where
    C: SynchronizeOutbound<S>,
    S: Clone,
{
    pub(crate) server: S,
    pub(crate) node: Node<
        C,
        InboundIdentifier,
        OutboundIdentifier,
        HashMapResources<InboundIdentifier, OutboundIdentifier>,
    >,
}

impl<C, S> Deref for Client<C, S>
where
    C: SynchronizeOutbound<S>,
    S: Clone,
{
    type Target = Node<
        C,
        InboundIdentifier,
        OutboundIdentifier,
        HashMapResources<InboundIdentifier, OutboundIdentifier>,
    >;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<C, S> DerefMut for Client<C, S>
where
    C: SynchronizeOutbound<S>,
    S: Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}

impl<C, S> Client<C, S>
where
    C: SynchronizeOutbound<S>,
    S: Clone,
{
    pub fn new(node: C, server: S) -> Self {
        Self {
            server,
            node: Node::new(node),
        }
    }

    pub fn synchronize_outbound(&mut self) {
        let resources = &mut self.node.resources.outbound_resources;
        let server = &self.server;
        let node = &mut self.node.node;

        for (identifier, resource) in resources.iter_mut() {
            if resource.is_dirty() {
                let generation = resource.generation();
                let packet = ResourcePacket {
                    identifier: identifier.clone(),
                    data: resource.serialize(generation.is_none()).unwrap(),
                };
                let payload = bincode::serialize(&packet).unwrap();
                node.synchronize(server.clone(), payload.clone());
                resource.snapshot();
                resource.set_dirty(false);
            }
        }
    }

    pub fn synchronize_inbound(&mut self, data: Vec<u8>) {
        let packet = bincode::deserialize::<ResourcePacket>(&data);
        if let Ok(packet) = packet {
            self.node
                .resources_mut()
                .inbound_resources
                .get_mut(&packet.identifier)
                .unwrap()
                .deserialize(&packet.data);
        }
    }
}

pub type InboundIdentifier = String;

pub type OutboundIdentifier = String;
