
#![deny(missing_docs)]
#![deny(warnings)]

use core::marker::PhantomData;
use core::ops::Deref;
use core::slice;

/// A circular buffer
pub struct CircularBuffer<T, A>
where
    A: AsMut<[T]> + AsRef<[T]>,
    T: Copy,
{
    _marker: PhantomData<[T]>,
    array: A,
    index: usize,
    len: usize,
}

impl<T, A> CircularBuffer<T, A>
where
    A: AsMut<[T]> + AsRef<[T]>,
    T: Copy,
{
    /// Creates a new empty circular buffer using `array` as backup storage.
    ///
    /// # Example
    /// ```
    /// #![feature(const_fn)]
    /// use heapless::CircularBuffer;
    /// const LEN: usize = 16;
    /// // with let
    /// let buf_let = CircularBuffer::new([0;LEN]);
    /// // with const
    /// const BUF_CONST: CircularBuffer<u8, [u8; LEN]>
    ///     = CircularBuffer::new([0;LEN]);
    /// // with static
    /// static BUF_STATIC: CircularBuffer<u8, [u8; LEN]>
    ///     = CircularBuffer::new([0;LEN]);
    /// // with slice
    /// let mut array = [0_usize; 8];
    /// let mut buf_slice = CircularBuffer::new(&mut array);
    /// ```
    pub const fn new(array: A) -> Self {
        CircularBuffer {
            _marker: PhantomData,
            array: array,
            index: 0,
            len: 0,
        }
    }

    /// Creates a new circular buffer using `array` as backup storage.
    /// Keeps the values and the length of the array.
    /// The backup storage is completly used upon creation.
    /// Insertions will overwrite at the start of the array.
    ///
    /// # Example
    /// ```
    /// use heapless::CircularBuffer;
    /// let buf = CircularBuffer::from_array([1,2,3]);
    /// assert!(buf.is_full());
    /// ```
    pub fn from_array(array: A) -> Self {
        let len = array.as_ref().len();
        CircularBuffer {
            _marker: PhantomData,
            array: array,
            index: 0,
            len: len
        }
    }

    /// Returns the capacity of this buffer.
    ///
    ///
    /// # Example
    /// ```
    /// use heapless::CircularBuffer;
    /// const LEN: usize = 16;
    /// let buf = CircularBuffer::new([0;LEN]);
    /// assert_eq!(buf.capacity(), LEN);
    /// ```
    pub fn capacity(&self) -> usize {
        self.array.as_ref().len()
    }

    /// Returns the current length of this buffer.
    ///
    /// # Example
    /// ```
    /// use heapless::CircularBuffer;
    /// let buf = CircularBuffer::from_array([1,2,3]);
    /// assert_eq!(buf.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the current index of this buffer.
    /// This is the next place to push to.
    ///
    /// # Example
    /// ```
    /// use heapless::CircularBuffer;
    /// let buf = CircularBuffer::from_array([1,2,3]);
    /// assert_eq!(buf.index(), 0);
    /// ```
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns `true` if the buffer is empty.
    ///
    /// # Example
    /// ```
    /// use heapless::CircularBuffer;
    /// let buf = CircularBuffer::new([0;8]);
    /// assert!(buf.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the buffer is saturated.
    ///
    /// Note that elements can still be inserted but will overwrite
    /// the last ones.
    ///
    /// # Example
    /// ```
    /// use heapless::CircularBuffer;
    /// let buf = CircularBuffer::from_array([1,2,3]);
    /// assert!(buf.is_full());
    /// ```
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// Pushes `element` into the buffer.
    ///
    /// This will overwrite an old value if the buffer is full.
    pub fn push(&mut self, element: T) {
        let len = self.len();
        let capacity = self.capacity();
        let index = self.index();
        if len < capacity {
            unsafe { self.set_len(len + 1); }
        }
        {
            let slice = self.array.as_mut();
            slice[index] = element;
        }
        unsafe { self.set_index(index + 1); }
    }

    /// Sets the length of the buffer.
    ///
    /// This is unsafe because no checks are performed, memory can be leaked
    /// or uninitialized memory may be exposed.
    unsafe fn set_len(&mut self, len: usize) {
        self.len = len;
    }

    /// Sets the index of the buffer with wrap-around.
    /// This is the next place to insert to.
    ///
    /// This is unsafe because memory can be leaked
    /// or uninitialized memory may be exposed.
    unsafe fn set_index(&mut self, index: usize) {
        self.index = index % self.capacity();
    }
}

impl<T, A> Deref for CircularBuffer<T, A>
where
    A: AsMut<[T]> + AsRef<[T]>,
    T: Copy,
{
    type Target = [T];

    fn deref(&self) -> &[T] {
        let slice = self.array.as_ref();

        if self.len == slice.len() {
            slice
        } else {
            unsafe { slice::from_raw_parts(slice.as_ptr(), self.len) }
        }
    }
}

#[cfg(test)]
mod test {

}
