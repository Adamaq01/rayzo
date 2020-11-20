use std::marker::PhantomData;
use std::{
    hash::Hash,
    ops::{Deref, DerefMut},
};

use crate::resources::Resources;

pub struct Node<N, I, O, R>
where
    I: Eq + Hash,
    O: Eq + Hash,
    R: Resources<I, O>,
{
    pub(crate) node: N,
    pub(crate) resources: R,
    _phantom: PhantomData<(I, O)>,
}

impl<N, I, O, R> Node<N, I, O, R>
where
    I: Eq + Hash,
    O: Eq + Hash,
    R: Resources<I, O>,
{
    pub fn new(node: N) -> Self {
        Self {
            node,
            resources: R::default(),
            _phantom: PhantomData::default(),
        }
    }

    pub fn resources(&self) -> &R {
        &self.resources
    }

    pub fn resources_mut(&mut self) -> &mut R {
        &mut self.resources
    }
}

impl<N, I, O, R> Deref for Node<N, I, O, R>
where
    I: Eq + Hash,
    O: Eq + Hash,
    R: Resources<I, O>,
{
    type Target = N;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<N, I, O, R> DerefMut for Node<N, I, O, R>
where
    I: Eq + Hash,
    O: Eq + Hash,
    R: Resources<I, O>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}
