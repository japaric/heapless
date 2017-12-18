//! Ring buffer
//!
//! Note: the current implementation will only work correctly on single-core devices.

use core::cell::UnsafeCell;
use core::marker::{PhantomData, Unsize};
use core::{intrinsics, ptr};

use untagged_option::UntaggedOption;

pub use self::spsc::{Consumer, Producer};
use BufferFullError;

mod spsc;

// AtomicUsize with no CAS operations that works on targets that have "no atomic support" according
// to their specification
struct AtomicUsize {
    v: UnsafeCell<usize>,
}

impl AtomicUsize {
    pub const fn new(v: usize) -> AtomicUsize {
        AtomicUsize {
            v: UnsafeCell::new(v),
        }
    }

    pub fn get_mut(&mut self) -> &mut usize {
        unsafe { &mut *self.v.get() }
    }

    pub fn load_acquire(&self) -> usize {
        unsafe { intrinsics::atomic_load_acq(self.v.get()) }
    }

    pub fn load_relaxed(&self) -> usize {
        unsafe { intrinsics::atomic_load_relaxed(self.v.get()) }
    }

    pub fn store_release(&self, val: usize) {
        unsafe { intrinsics::atomic_store_rel(self.v.get(), val) }
    }
}

/// An statically allocated ring buffer backed by an array `A`
///
/// Note: the current implementation will only work correctly on single-core devices.

pub struct RingBuffer<T, A>
where
    // FIXME(rust-lang/rust#44580) use "const generics" instead of `Unsize`
    A: Unsize<[T]>,
{
    _marker: PhantomData<[T]>,

    // this is from where we dequeue items
    head: AtomicUsize,

    // this is where we enqueue new items
    tail: AtomicUsize,

    buffer: UntaggedOption<A>,
}

impl<T, A> RingBuffer<T, A>
where
    A: Unsize<[T]>,
{
    /// Creates an empty ring buffer with capacity equals to the length of the array `A` *minus
    /// one*.
    pub const fn new() -> Self {
        RingBuffer {
            _marker: PhantomData,
            buffer: UntaggedOption::none(),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Returns the maximum number of elements the ring buffer can hold
    pub fn capacity(&self) -> usize {
        let buffer: &[T] = unsafe { self.buffer.as_ref() };
        buffer.len() - 1
    }

    /// Returns the item in the front of the queue, or `None` if the queue is empty
    pub fn dequeue(&mut self) -> Option<T> {
        let n = self.capacity() + 1;

        let head = self.head.get_mut();
        let tail = self.tail.get_mut();

        let buffer: &[T] = unsafe { self.buffer.as_ref() };

        if *head != *tail {
            let item = unsafe { ptr::read(buffer.get_unchecked(*head)) };
            *head = (*head + 1) % n;
            Some(item)
        } else {
            None
        }
    }

    /// Adds an `item` to the end of the queue
    ///
    /// Returns `BufferFullError` if the queue is full
    pub fn enqueue(&mut self, item: T) -> Result<(), BufferFullError> {
        let n = self.capacity() + 1;

        let head = self.head.get_mut();
        let tail = self.tail.get_mut();

        let buffer: &mut [T] = unsafe { self.buffer.as_mut() };

        let next_tail = (*tail + 1) % n;
        if next_tail != *head {
            // NOTE(ptr::write) the memory slot that we are about to write to is uninitialized. We
            // use `ptr::write` to avoid running `T`'s destructor on the uninitialized memory
            unsafe { ptr::write(buffer.get_unchecked_mut(*tail), item) }
            *tail = next_tail;
            Ok(())
        } else {
            Err(BufferFullError)
        }
    }

    /// Returns the number of elements in the queue
    pub fn len(&self) -> usize {
        let head = self.head.load_relaxed();
        let tail = self.tail.load_relaxed();

        if head > tail {
            head - tail
        } else {
            tail - head
        }
    }

    /// Returns `true` if the ring buffer has a length of 0
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterates from the front of the queue to the back
    pub fn iter(&self) -> Iter<T, A> {
        Iter {
            rb: self,
            index: 0,
            len: self.len(),
        }
    }

    /// Returns an iterator that allows modifying each value.
    pub fn iter_mut(&mut self) -> IterMut<T, A> {
        let len = self.len();
        IterMut {
            rb: self,
            index: 0,
            len,
        }
    }
}

impl<T, A> Drop for RingBuffer<T, A>
where
    A: Unsize<[T]>,
{
    fn drop(&mut self) {
        for item in self {
            unsafe {
                ptr::drop_in_place(item);
            }
        }
    }
}

impl<'a, T, A> IntoIterator for &'a RingBuffer<T, A>
where
    A: Unsize<[T]>,
{
    type Item = &'a T;
    type IntoIter = Iter<'a, T, A>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, A> IntoIterator for &'a mut RingBuffer<T, A>
where
    A: Unsize<[T]>,
{
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T, A>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An iterator over a ring buffer items
pub struct Iter<'a, T, A>
where
    A: Unsize<[T]> + 'a,
    T: 'a,
{
    rb: &'a RingBuffer<T, A>,
    index: usize,
    len: usize,
}

/// A mutable iterator over a ring buffer items
pub struct IterMut<'a, T, A>
where
    A: Unsize<[T]> + 'a,
    T: 'a,
{
    rb: &'a mut RingBuffer<T, A>,
    index: usize,
    len: usize,
}

impl<'a, T, A> Iterator for Iter<'a, T, A>
where
    A: Unsize<[T]> + 'a,
    T: 'a,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if self.index < self.len {
            let head = self.rb.head.load_relaxed();

            let buffer: &[T] = unsafe { self.rb.buffer.as_ref() };
            let ptr = buffer.as_ptr();
            let i = (head + self.index) % (self.rb.capacity() + 1);
            self.index += 1;
            Some(unsafe { &*ptr.offset(i as isize) })
        } else {
            None
        }
    }
}

impl<'a, T, A> Iterator for IterMut<'a, T, A>
where
    A: Unsize<[T]> + 'a,
    T: 'a,
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        if self.index < self.len {
            let head = self.rb.head.load_relaxed();

            let capacity = self.rb.capacity() + 1;
            let buffer: &mut [T] = unsafe { self.rb.buffer.as_mut() };
            let ptr: *mut T = buffer.as_mut_ptr();
            let i = (head + self.index) % capacity;
            self.index += 1;
            Some(unsafe { &mut *ptr.offset(i as isize) })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use RingBuffer;

    #[test]
    fn drop() {
        struct Droppable;
        impl Droppable {
            fn new() -> Self {
                unsafe {
                    COUNT += 1;
                }
                Droppable
            }
        }
        impl Drop for Droppable {
            fn drop(&mut self) {
                unsafe {
                    COUNT -= 1;
                }
            }
        }

        static mut COUNT: i32 = 0;


        {
            let mut v: RingBuffer<Droppable, [Droppable; 4]> = RingBuffer::new();
            v.enqueue(Droppable::new()).unwrap();
            v.enqueue(Droppable::new()).unwrap();
            v.dequeue().unwrap();
        }

        assert_eq!(unsafe { COUNT }, 0);

        {
            let mut v: RingBuffer<Droppable, [Droppable; 4]> = RingBuffer::new();
            v.enqueue(Droppable::new()).unwrap();
            v.enqueue(Droppable::new()).unwrap();
        }

        assert_eq!(unsafe { COUNT }, 0);
    }

    #[test]
    fn full() {
        let mut rb: RingBuffer<i32, [i32; 4]> = RingBuffer::new();

        rb.enqueue(0).unwrap();
        rb.enqueue(1).unwrap();
        rb.enqueue(2).unwrap();

        assert!(rb.enqueue(3).is_err());
    }

    #[test]
    fn iter() {
        let mut rb: RingBuffer<i32, [i32; 4]> = RingBuffer::new();

        rb.enqueue(0).unwrap();
        rb.enqueue(1).unwrap();
        rb.enqueue(2).unwrap();

        let mut items = rb.iter();

        assert_eq!(items.next(), Some(&0));
        assert_eq!(items.next(), Some(&1));
        assert_eq!(items.next(), Some(&2));
        assert_eq!(items.next(), None);
    }

    #[test]
    fn iter_mut() {
        let mut rb: RingBuffer<i32, [i32; 4]> = RingBuffer::new();

        rb.enqueue(0).unwrap();
        rb.enqueue(1).unwrap();
        rb.enqueue(2).unwrap();

        let mut items = rb.iter_mut();

        assert_eq!(items.next(), Some(&mut 0));
        assert_eq!(items.next(), Some(&mut 1));
        assert_eq!(items.next(), Some(&mut 2));
        assert_eq!(items.next(), None);
    }

    #[test]
    fn sanity() {
        let mut rb: RingBuffer<i32, [i32; 4]> = RingBuffer::new();

        assert_eq!(rb.dequeue(), None);

        rb.enqueue(0).unwrap();

        assert_eq!(rb.dequeue(), Some(0));

        assert_eq!(rb.dequeue(), None);
    }

    #[test]
    fn wrap_around() {
        let mut rb: RingBuffer<i32, [i32; 4]> = RingBuffer::new();

        rb.enqueue(0).unwrap();
        rb.enqueue(1).unwrap();
        rb.enqueue(2).unwrap();
        rb.dequeue().unwrap();
        rb.dequeue().unwrap();
        rb.dequeue().unwrap();
        rb.enqueue(3).unwrap();
        rb.enqueue(4).unwrap();

        assert_eq!(rb.len(), 2);
    }
}
