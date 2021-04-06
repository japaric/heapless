//! Unfortunate implementation detail required to construct `heapless` types in const context

use core::{marker::PhantomData, mem::MaybeUninit};

#[cfg(has_atomics)]
use crate::spsc::{Atomic, MultiCore};

/// `const-fn` version of [`BinaryHeap`](../binary_heap/struct.BinaryHeap.html)
pub struct BinaryHeap<A, K, U = usize> {
    pub(crate) _kind: PhantomData<K>,
    pub(crate) data: Vec<A, U>,
}

/// `const-fn` version of [`LinearMap`](../struct.LinearMap.html)
pub struct LinearMap<A, U = usize> {
    pub(crate) buffer: Vec<A, U>,
}

/// `const-fn` version of [`spsc::Queue`](../spsc/struct.Queue.html)
#[cfg(has_atomics)]
pub struct Queue<A, U = usize, C = MultiCore> {
    // this is from where we dequeue items
    pub(crate) head: Atomic<U, C>,

    // this is where we enqueue new items
    pub(crate) tail: Atomic<U, C>,

    pub(crate) buffer: MaybeUninit<A>,
}

/// `const-fn` version of [`String`](../struct.String.html)
pub struct String<A, U = usize> {
    pub(crate) vec: Vec<A, U>,
}

/// `const-fn` version of [`Vec`](../struct.Vec.html)
pub struct Vec<A, U = usize> {
    pub(crate) buffer: MaybeUninit<A>,
    pub(crate) len: U,
}