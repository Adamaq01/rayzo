use downcast_rs::{impl_downcast, Downcast};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_diff::{Apply, Diff, SerdeDiff};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

pub struct HashMapResources<I, O>
where
    I: Eq + Hash,
    O: Eq + Hash,
{
    pub(crate) inbound_resources: HashMap<I, Box<dyn InternalInboundResource>>,
    pub(crate) outbound_resources: HashMap<O, Box<dyn InternalOutboundResource>>,
}

impl<I, O> Default for HashMapResources<I, O>
where
    I: Eq + Hash,
    O: Eq + Hash,
{
    fn default() -> Self {
        Self {
            inbound_resources: HashMap::new(),
            outbound_resources: HashMap::new(),
        }
    }
}

impl<I, O> Resources<I, O> for HashMapResources<I, O>
where
    I: Eq + Hash,
    O: Eq + Hash,
{
    fn register_inbound<T>(&mut self, identifier: I, resource: T)
    where
        T: 'static + Debug + DeserializeOwned + SerdeDiff + Send,
    {
        self.inbound_resources
            .insert(identifier, Box::new(InboundResource::new(resource)));
    }

    fn register_outbound<T>(&mut self, identifier: O, resource: T)
    where
        T: 'static + Debug + Clone + Serialize + SerdeDiff + Send,
    {
        self.outbound_resources
            .insert(identifier, Box::new(OutboundResource::new(resource)));
    }

    fn inbound<T>(&self, identifier: I) -> Option<&InboundResource<T>>
    where
        T: 'static + Debug + DeserializeOwned + SerdeDiff + Send,
    {
        self.inbound_resources
            .get(&identifier)
            .and_then(|b| b.downcast_ref::<InboundResource<T>>())
    }

    fn outbound<T>(&self, identifier: O) -> Option<&OutboundResource<T>>
    where
        T: 'static + Debug + Clone + Serialize + SerdeDiff + Send,
    {
        self.outbound_resources
            .get(&identifier)
            .and_then(|b| b.downcast_ref::<OutboundResource<T>>())
    }

    fn outbound_mut<T>(&mut self, identifier: O) -> Option<&mut OutboundResource<T>>
    where
        T: 'static + Debug + Clone + Serialize + SerdeDiff + Send,
    {
        self.outbound_resources
            .get_mut(&identifier)
            .and_then(|b| b.downcast_mut::<OutboundResource<T>>())
    }
}

pub trait Resources<I, O>: Default {
    fn register_inbound<T>(&mut self, identifier: I, resource: T)
    where
        T: 'static + Debug + DeserializeOwned + SerdeDiff + Send;

    fn register_outbound<T>(&mut self, identifier: O, resource: T)
    where
        T: 'static + Debug + Clone + Serialize + SerdeDiff + Send;

    fn inbound<T>(&self, identifier: I) -> Option<&InboundResource<T>>
    where
        T: 'static + Debug + DeserializeOwned + SerdeDiff + Send;

    fn outbound<T>(&self, identifier: O) -> Option<&OutboundResource<T>>
    where
        T: 'static + Debug + Clone + Serialize + SerdeDiff + Send;

    fn outbound_mut<T>(&mut self, identifier: O) -> Option<&mut OutboundResource<T>>
    where
        T: 'static + Debug + Clone + Serialize + SerdeDiff + Send;
}

pub struct InboundResource<T>
where
    T: Debug + DeserializeOwned + SerdeDiff,
{
    data: T,
}

impl<T> InboundResource<T>
where
    T: Debug + DeserializeOwned + SerdeDiff,
{
    pub(crate) fn new(data: T) -> InboundResource<T> {
        Self { data }
    }
}

impl<T> Deref for InboundResource<T>
where
    T: Debug + DeserializeOwned + SerdeDiff,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

pub(crate) struct Snapshot<T>
where
    T: Debug + Clone + Serialize + SerdeDiff,
{
    data: T,
    generation: usize,
}

impl<T> Snapshot<T>
where
    T: Debug + Clone + Serialize + SerdeDiff,
{
    fn new(data: T, generation: usize) -> Self {
        Self { data, generation }
    }
}

pub struct OutboundResource<T>
where
    T: Debug + Clone + Serialize + SerdeDiff,
{
    snapshot: Option<Snapshot<T>>,
    dirty: bool,
    data: T,
}

impl<T> OutboundResource<T>
where
    T: Debug + Clone + Serialize + SerdeDiff,
{
    pub(crate) fn new(data: T) -> OutboundResource<T> {
        Self {
            snapshot: None,
            dirty: false,
            data,
        }
    }
}

impl<T> Deref for OutboundResource<T>
where
    T: Debug + Clone + Serialize + SerdeDiff,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for OutboundResource<T>
where
    T: Debug + Clone + Serialize + SerdeDiff,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty = true;
        &mut self.data
    }
}

pub(crate) trait InternalInboundResource: Downcast + Send {
    fn deserialize(&mut self, data: &Vec<u8>);
}

pub(crate) trait InternalOutboundResource: Downcast + Send {
    fn is_dirty(&self) -> bool;
    fn set_dirty(&mut self, dirty: bool);
    fn snapshot(&mut self);
    fn generation(&self) -> Option<usize>;
    fn serialize(&self, full: bool) -> Option<Vec<u8>>;
}

impl_downcast!(InternalInboundResource);
impl_downcast!(InternalOutboundResource);

impl<T: 'static> InternalOutboundResource for OutboundResource<T>
where
    T: Debug + Clone + Serialize + SerdeDiff + Send,
{
    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    fn snapshot(&mut self) {
        let generation = self.snapshot.as_ref().map_or(0, |s| s.generation + 1);
        self.snapshot = Some(Snapshot::new(self.data.clone(), generation));
    }

    fn generation(&self) -> Option<usize> {
        self.snapshot.as_ref().map(|s| s.generation)
    }

    fn serialize(&self, full: bool) -> Option<Vec<u8>> {
        if full || self.snapshot.is_none() {
            let data = rmp_serde::to_vec(&self.data).unwrap();
            Some(rmp_serde::to_vec(&SerializedResource::Full(data)).unwrap())
        } else {
            let snapshot = self.snapshot.as_ref().unwrap();
            let diff = Diff::serializable(&snapshot.data, &self.data);
            let diff_serialized = rmp_serde::to_vec(&diff).unwrap();
            if diff.has_changes() {
                let data = rmp_serde::to_vec(&self.data).unwrap();
                if data.len() > diff_serialized.len() {
                    Some(rmp_serde::to_vec(&SerializedResource::Diff(diff_serialized)).unwrap())
                } else {
                    Some(rmp_serde::to_vec(&SerializedResource::Full(data)).unwrap())
                }
            } else {
                None
            }
        }
    }
}

impl<T: 'static> InternalInboundResource for InboundResource<T>
where
    T: Debug + DeserializeOwned + SerdeDiff + Send,
{
    fn deserialize(&mut self, data: &Vec<u8>) {
        let data = rmp_serde::from_slice::<SerializedResource>(data.as_slice()).unwrap();
        match data {
            SerializedResource::Diff(diff) => {
                let mut diff = rmp_serde::Deserializer::new(diff.as_slice());
                Apply::apply(&mut diff, &mut self.data).unwrap();
            }
            SerializedResource::Full(full) => {
                self.data = rmp_serde::from_slice::<T>(&full).unwrap()
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
enum SerializedResource {
    Diff(Vec<u8>),
    Full(Vec<u8>),
}
