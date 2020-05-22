use core::{fmt, hash, iter::FromIterator, mem::MaybeUninit, ops, ptr, slice};

use generic_array::{
    ArrayLength,
    GenericArray,
    typenum::{IsGreaterOrEqual, True},
};
use hash32;

impl<A> crate::i::Vec<A> {
    /// `Vec` `const` constructor; wrap the returned value in [`Vec`](../struct.Vec.html)
    pub const fn new() -> Self {
        Self {
            buffer: MaybeUninit::uninit(),
            len: 0,
        }
    }
}

impl<T, N> crate::i::Vec<GenericArray<T, N>>
where
    N: ArrayLength<T>,
{
    pub(crate) fn as_slice(&self) -> &[T] {
        // NOTE(unsafe) avoid bound checks in the slicing operation
        // &buffer[..self.len]
        unsafe { slice::from_raw_parts(self.buffer.as_ptr() as *const T, self.len) }
    }

    pub(crate) fn as_mut_slice(&mut self) -> &mut [T] {
        // NOTE(unsafe) avoid bound checks in the slicing operation
        // &mut buffer[..len]
        unsafe { slice::from_raw_parts_mut(self.buffer.as_mut_ptr() as *mut T, self.len) }
    }

    pub(crate) fn capacity(&self) -> usize {
        N::to_usize()
    }

    pub(crate) fn clear(&mut self) {
        self.truncate(0);
    }

    pub(crate) fn clone(&self) -> Self
    where
        T: Clone,
    {
        let mut new = Self::new();
        new.extend_from_slice(self.as_slice()).unwrap();
        new
    }

    pub(crate) fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for elem in iter {
            self.push(elem).ok().unwrap()
        }
    }

    pub(crate) fn extend_from_slice(&mut self, other: &[T]) -> Result<(), ()>
    where
        T: Clone,
    {
        if self.len + other.len() > self.capacity() {
            // won't fit in the `Vec`; don't modify anything and return an error
            Err(())
        } else {
            for elem in other {
                unsafe {
                    self.push_unchecked(elem.clone());
                }
            }
            Ok(())
        }
    }

    pub(crate) fn is_full(&self) -> bool {
        self.len == self.capacity()
    }

    pub(crate) unsafe fn pop_unchecked(&mut self) -> T {
        debug_assert!(!self.as_slice().is_empty());

        self.len -= 1;
        (self.buffer.as_ptr() as *const T).add(self.len).read()
    }

    pub(crate) fn push(&mut self, item: T) -> Result<(), T> {
        if self.len < self.capacity() {
            unsafe { self.push_unchecked(item) }
            Ok(())
        } else {
            Err(item)
        }
    }

    pub(crate) unsafe fn push_unchecked(&mut self, item: T) {
        // NOTE(ptr::write) the memory slot that we are about to write to is uninitialized. We
        // use `ptr::write` to avoid running `T`'s destructor on the uninitialized memory
        (self.buffer.as_mut_ptr() as *mut T)
            .add(self.len)
            .write(item);

        self.len += 1;
    }

    pub(crate) fn insert(&mut self, index: usize, item: T) -> Result<(), T> {
        let len = self.len;
        assert!(index <= len);
        if self.len < self.capacity() && index <= self.len {
            unsafe { self.insert_unchecked(index, item) }
            Ok(())
        } else {
            Err(item)
        }
    }

    pub(crate) unsafe fn insert_unchecked(&mut self, index: usize, item: T) {
        let p = (self.buffer.as_mut_ptr() as *mut T).add(index);
        // Shift everything over to make space. (Duplicating the
        // `index`th element into two consecutive places.)
        ptr::copy(p, p.offset(1), self.len - index);
        // Write it in, overwriting the first copy of the `index`th
        // element.
        ptr::write(p, item);

        self.len += 1;
    }

    pub(crate) fn remove(&mut self, index: usize) -> Result<T, ()> {
        if index < self.len {
            unsafe { Ok(self.remove_unchecked(index)) }
        } else {
            Err(())
        }
    }

    pub(crate) unsafe fn remove_unchecked(&mut self, index: usize) -> T {
        // the place we are taking from.
        let p = (self.buffer.as_mut_ptr() as *mut T).add(index);

        // copy it out, unsafely having a copy of the value on
        // the stack and in the vector at the same time.
        let ret = ptr::read(p);

        // shift everything down to fill in that spot.
        ptr::copy(p.offset(1), p, self.len - index - 1);

        self.len -= 1;
        ret
    }

    unsafe fn swap_remove_unchecked(&mut self, index: usize) -> T {
        let length = self.len;
        debug_assert!(index < length);
        ptr::swap(
            self.as_mut_slice().get_unchecked_mut(index),
            self.as_mut_slice().get_unchecked_mut(length - 1),
        );
        self.pop_unchecked()
    }

    pub(crate) fn swap_remove(&mut self, index: usize) -> T {
        assert!(index < self.len);
        unsafe { self.swap_remove_unchecked(index) }
    }

    pub(crate) fn truncate(&mut self, len: usize) {
        unsafe {
            // drop any extra elements
            while len < self.len {
                // decrement len before the drop_in_place(), so a panic on Drop
                // doesn't re-drop the just-failed value.
                self.len -= 1;
                let len = self.len;
                ptr::drop_in_place(self.as_mut_slice().get_unchecked_mut(len));
            }
        }
    }
}

/// A fixed capacity [`Vec`](https://doc.rust-lang.org/std/vec/struct.Vec.html)
///
/// # Examples
///
/// ```
/// use heapless::Vec;
/// use heapless::consts::*;
///
/// // A vector with a fixed capacity of 8 elements allocated on the stack
/// let mut vec = Vec::<_, U8>::new();
/// vec.push(1);
/// vec.push(2);
///
/// assert_eq!(vec.len(), 2);
/// assert_eq!(vec[0], 1);
///
/// assert_eq!(vec.pop(), Some(2));
/// assert_eq!(vec.len(), 1);
///
/// vec[0] = 7;
/// assert_eq!(vec[0], 7);
///
/// vec.extend([1, 2, 3].iter().cloned());
///
/// for x in &vec {
///     println!("{}", x);
/// }
/// assert_eq!(vec, [7, 1, 2, 3]);
/// ```
pub struct Vec<T, N>(#[doc(hidden)] pub crate::i::Vec<GenericArray<T, N>>)
where
    N: ArrayLength<T>;

impl<T, N> Clone for Vec<T, N>
where
    N: ArrayLength<T>,
    T: Clone,
{
    fn clone(&self) -> Self {
        Vec(self.0.clone())
    }
}

impl<T, N> Vec<T, N>
where
    N: ArrayLength<T>,
{
    /* Constructors */
    /// Constructs a new, empty vector with a fixed capacity of `N`
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::Vec;
    /// use heapless::consts::*;
    ///
    /// // allocate the vector on the stack
    /// let mut x: Vec<u8, U16> = Vec::new();
    ///
    /// // allocate the vector in a static variable
    /// static mut X: Vec<u8, U16> = Vec(heapless::i::Vec::new());
    /// ```
    pub fn new() -> Self {
        Vec(crate::i::Vec::new())
    }

    /// APIs modeled after [`std::io::Write`] offer an interface of the form
    /// `write(&mut [u8]) -> Result<usize, E>`, with the contract that the
    /// Ok value signals how many bytes were written, of at most length of
    /// the buffer.
    ///
    /// This constructor allows wrapping such interfaces in a more ergonomic way,
    /// returning a new byte buffer filled using the writer.
    ///
    /// [`std::io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    pub fn from_writer<E>(
        write: impl FnOnce(&mut [T]) -> core::result::Result<usize, E>
    )
        -> core::result::Result<Self, E>
    where
        T: Clone + Default,
    {
        let mut new = Self::new();
        new.resize_to_capacity();

        let result = write(&mut new);

        result.map(|count| {
            new.truncate(count);
            new
        })
    }

    /// Returns an immutable slice view.
    // Add as inherent method as it's annoying to import AsSlice.
    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    /// Returns a mutable slice view.
    // Add as inherent method as it's annoying to import AsSlice.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.0.as_mut_slice()
    }

    /// Constructs a new vector with a fixed capacity of `N` and fills it
    /// with the provided slice.
    ///
    /// This is equivalent to the following code:
    ///
    /// ```
    /// use heapless::Vec;
    /// use heapless::consts::*;
    ///
    /// let mut v: Vec<u8, U16> = Vec::new();
    /// v.extend_from_slice(&[1, 2, 3]);
    /// ```
    #[inline]
    pub fn from_slice(other: &[T]) -> Result<Self, ()>
    where
        T: Clone,
    {
        let mut v = Vec::new();
        v.extend_from_slice(other)?;
        Ok(v)
    }

    /* Public API */
    /// Returns the maximum number of elements the vector can hold
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Clears the vector, removing all values.
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Clones and appends all elements in a slice to the `Vec`.
    ///
    /// Iterates over the slice `other`, clones each element, and then appends
    /// it to this `Vec`. The `other` vector is traversed in-order.
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::Vec;
    /// use heapless::consts::*;
    ///
    /// let mut vec = Vec::<u8, U8>::new();
    /// vec.push(1).unwrap();
    /// vec.extend_from_slice(&[2, 3, 4]).unwrap();
    /// assert_eq!(*vec, [1, 2, 3, 4]);
    /// ```
    pub fn extend_from_slice(&mut self, other: &[T]) -> Result<(), ()>
    where
        T: Clone,
    {
        self.0.extend_from_slice(other)
    }

    // cf. https://internals.rust-lang.org/t/add-vec-insert-slice-at-to-insert-the-content-of-a-slice-at-an-arbitrary-index/11008/3
    /// Insert slice at given index, if capacity allows it.
    pub fn insert_slice_at(&mut self, slice: &[T], at: usize) -> core::result::Result<(), ()>
    where
        T: Copy + Default
    {
        let l = slice.len();
        let before = self.len();

        // make space
        self.resize_default(before + l)?;

        // move back existing
        let raw: &mut [T] = self.as_mut_slice();
        // if/when MSRV is raised (from 1.36) to 1.37, use builtin method:
        // raw.copy_within(at..before, at + l);
        unsafe {
            let p = &mut raw[0] as *mut T;
            core::ptr::copy(p.add(at), p.add(at + l), l);
        }

        // insert slice
        raw[at..][..l].copy_from_slice(slice);

        Ok(())
    }


    /// Clone into at least same size vector.
    // We can't implement Into since it would clash with blanket implementations.
    pub fn to_vec<M>(&self) -> Vec<T, M>
    where
        M: ArrayLength<T> + IsGreaterOrEqual<N, Output = True>,
        T: Clone,
    {
        match Vec::from_slice(self) {
            Ok(vec) => vec,
            _ => unreachable!(),
        }
    }

    /// Fallible clone into differently sized vector.
    // We can't implement TryInto since it would clash with blanket implementations.
    pub fn try_to_vec<M>(&self) -> Result<Vec<T, M>, ()>
    where
        M: ArrayLength<T>,
        T: Clone,
    {
        Vec::from_slice(self)
    }

    /// Removes the last element from a vector and return it, or `None` if it's empty
    pub fn pop(&mut self) -> Option<T> {
        if self.0.len != 0 {
            Some(unsafe { self.0.pop_unchecked() })
        } else {
            None
        }
    }

    /// Appends an `item` to the back of the collection
    ///
    /// Returns back the `item` if the vector is full
    pub fn push(&mut self, item: T) -> Result<(), T> {
        self.0.push(item)
    }

    pub(crate) unsafe fn push_unchecked(&mut self, item: T) {
        self.0.push_unchecked(item)
    }

    /// Shortens the vector, keeping the first `len` elements and dropping the rest.
    pub fn truncate(&mut self, len: usize) {
        unsafe {
            // drop any extra elements
            while len < self.len() {
                // decrement len before the drop_in_place(), so a panic on Drop
                // doesn't re-drop the just-failed value.
                self.0.len -= 1;
                let len = self.len();
                ptr::drop_in_place(self.get_unchecked_mut(len));
            }
        }
    }

    /// Resizes the Vec in-place so that len is equal to new_len.
    ///
    /// If new_len is greater than len, the Vec is extended by the
    /// difference, with each additional slot filled with value. If
    /// new_len is less than len, the Vec is simply truncated.
    ///
    /// See also [`resize_default`](struct.Vec.html#method.resize_default).
    pub fn resize(&mut self, new_len: usize, value: T) -> Result<(), ()>
    where
        T: Clone,
    {
        if new_len > self.capacity() {
            return Err(());
        }

        if new_len > self.len() {
            while self.len() < new_len {
                self.push(value.clone()).ok();
            }
        } else {
            self.truncate(new_len);
        }

        Ok(())
    }

    /// Resizes the `Vec` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `Vec` is extended by the
    /// difference, with each additional slot filled with `Default::default()`.
    /// If `new_len` is less than `len`, the `Vec` is simply truncated.
    ///
    /// See also [`resize`](struct.Vec.html#method.resize).
    pub fn resize_default(&mut self, new_len: usize) -> Result<(), ()>
    where
        T: Clone + Default,
    {
        self.resize(new_len, T::default())
    }

    /// Resizes the `Vec` in-place so that `len` is equal to `capacity`.
    // Useful because v.resize_default(v.capacity()) makes the borrow checker complain
    pub fn resize_to_capacity(&mut self)
    where
        T: Clone + Default,
    {
        self.resize(N::USIZE, T::default()).unwrap();
    }

    /// Inserts an item at position `index` within the vector, shifting all
    /// items after it to the right.
    ///
    /// Returns the item if index is out of bounds or there is no capacity.
    pub fn insert(&mut self, index: usize, item: T) -> Result<(), T> {
        self.0.insert(index, item)
    }


    /// Removes and returns the element at position `index` within the vector,
    /// shifting all elements after it to the left.
    ///
    /// Returns error if index is out of bounds.
    pub fn remove(&mut self, index: usize) -> Result<T, ()> {
        self.0.remove(index)
    }

    /// Removes an element from the vector and returns it.
    ///
    /// The removed element is replaced by the last element of the vector.
    ///
    /// This does not preserve ordering, but is O(1).
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::Vec;
    /// use heapless::consts::*;
    ///
    /// let mut v: Vec<_, U8> = Vec::new();
    /// v.push("foo").unwrap();
    /// v.push("bar").unwrap();
    /// v.push("baz").unwrap();
    /// v.push("qux").unwrap();
    ///
    /// assert_eq!(v.swap_remove(1), "bar");
    /// assert_eq!(&*v, ["foo", "qux", "baz"]);
    ///
    /// assert_eq!(v.swap_remove(0), "foo");
    /// assert_eq!(&*v, ["baz", "qux"]);
    /// ```
    pub fn swap_remove(&mut self, index: usize) -> T {
        self.0.swap_remove(index)
    }

    pub(crate) unsafe fn swap_remove_unchecked(&mut self, index: usize) -> T {
        self.0.swap_remove_unchecked(index)
    }

    pub(crate) fn is_full(&self) -> bool {
        self.0.is_full()
    }

    /// Returns `true` if `needle` is a prefix of the Vec.
    ///
    /// Always returns `true` if `needle` is an empty slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::Vec;
    /// use heapless::consts::*;
    ///
    /// let v: Vec<_, U8> = Vec::from_slice(b"abc").unwrap();
    /// assert_eq!(v.starts_with(b""), true);
    /// assert_eq!(v.starts_with(b"ab"), true);
    /// assert_eq!(v.starts_with(b"bc"), false);
    /// ```
    #[inline]
    pub fn starts_with(&self, needle: &[T]) -> bool
    where
        T: PartialEq,
    {
        let n = needle.len();
        self.len() >= n && needle == &self[..n]
    }

    /// Returns `true` if `needle` is a suffix of the Vec.
    ///
    /// Always returns `true` if `needle` is an empty slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::Vec;
    /// use heapless::consts::*;
    ///
    /// let v: Vec<_, U8> = Vec::from_slice(b"abc").unwrap();
    /// assert_eq!(v.ends_with(b""), true);
    /// assert_eq!(v.ends_with(b"ab"), false);
    /// assert_eq!(v.ends_with(b"bc"), true);
    /// ```
    #[inline]
    pub fn ends_with(&self, needle: &[T]) -> bool
    where
        T: PartialEq,
    {
        let (v, n) = (self.len(), needle.len());
        v >= n && needle == &self[v - n..]
    }
}

impl<N> Vec<u8, N>
where
    N: ArrayLength<u8>,
{
    /// Wrap same underlying buffer as byte buffer,
    /// consuming the vector.
    pub fn into_byte_buf(self) -> crate::ByteBuf<N> {
        crate::ByteBuf::from(self)
    }

    /// Clone into at least same size byte buffer.
    pub fn to_byte_buf<M>(&self) -> crate::ByteBuf<M>
    where
        M: ArrayLength<u8> + IsGreaterOrEqual<N, Output = True>,
    {
        match crate::ByteBuf::from_slice(self) {
            Ok(byte_buf) => byte_buf,
            _ => unreachable!(),
        }
    }

    /// Fallible conversion into differently sized byte buffer.
    pub fn try_to_byte_buf<M>(&self) -> Result<crate::ByteBuf<M>, ()>
    where
        M: ArrayLength<u8>,
    {
        crate::ByteBuf::from_slice(self)
    }
}

impl<T, N> Default for Vec<T, N>
where
    N: ArrayLength<T>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, N> fmt::Debug for Vec<T, N>
where
    T: fmt::Debug,
    N: ArrayLength<T>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <[T] as fmt::Debug>::fmt(self, f)
    }
}

impl<N> fmt::Write for Vec<u8, N>
where
    N: ArrayLength<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self.extend_from_slice(s.as_bytes()) {
            Ok(()) => Ok(()),
            Err(_) => Err(fmt::Error),
        }
    }
}

impl<T, N> Drop for Vec<T, N>
where
    N: ArrayLength<T>,
{
    fn drop(&mut self) {
        unsafe { ptr::drop_in_place(&mut self[..]) }
    }
}

impl<T, N> Extend<T> for Vec<T, N>
where
    N: ArrayLength<T>,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.0.extend(iter)
    }
}

impl<'a, T, N> Extend<&'a T> for Vec<T, N>
where
    T: 'a + Copy,
    N: ArrayLength<T>,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = &'a T>,
    {
        self.extend(iter.into_iter().cloned())
    }
}

impl<T, N> hash::Hash for Vec<T, N>
where
    T: core::hash::Hash,
    N: ArrayLength<T>,
{
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        <[T] as hash::Hash>::hash(self, state)
    }
}

impl<T, N> hash32::Hash for Vec<T, N>
where
    T: hash32::Hash,
    N: ArrayLength<T>,
{
    fn hash<H: hash32::Hasher>(&self, state: &mut H) {
        <[T] as hash32::Hash>::hash(self, state)
    }
}

impl<'a, T, N> IntoIterator for &'a Vec<T, N>
where
    N: ArrayLength<T>,
{
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, N> IntoIterator for &'a mut Vec<T, N>
where
    N: ArrayLength<T>,
{
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T, N> FromIterator<T> for Vec<T, N>
where
    N: ArrayLength<T>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut vec = Vec::new();
        for i in iter {
            vec.push(i).ok().expect("Vec::from_iter overflow");
        }
        vec
    }
}

/// An iterator that moves out of an [`Vec`][`Vec`].
///
/// This struct is created by calling the `into_iter` method on [`Vec`][`Vec`].
///
/// [`Vec`]: (https://doc.rust-lang.org/std/vec/struct.Vec.html)
///
pub struct IntoIter<T, N>
where
    N: ArrayLength<T>,
{
    vec: Vec<T, N>,
    next: usize,
}

impl<T, N> Iterator for IntoIter<T, N>
where
    N: ArrayLength<T>,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next < self.vec.len() {
            let item = unsafe {
                (self.vec.0.buffer.as_ptr() as *const T)
                    .add(self.next)
                    .read()
            };
            self.next += 1;
            Some(item)
        } else {
            None
        }
    }
}

impl<T, N> Clone for IntoIter<T, N>
where
    T: Clone,
    N: ArrayLength<T>,
{
    fn clone(&self) -> Self {
        Self {
            vec: self.vec.clone(),
            next: self.next,
        }
    }
}

impl<T, N> Drop for IntoIter<T, N>
where
    N: ArrayLength<T>,
{
    fn drop(&mut self) {
        unsafe {
            // Drop all the elements that have not been moved out of vec
            ptr::drop_in_place(&mut self.vec[self.next..]);
            // Prevent dropping of other elements
            self.vec.0.len = 0;
        }
    }
}

impl<T, N> IntoIterator for Vec<T, N>
where
    N: ArrayLength<T>,
{
    type Item = T;
    type IntoIter = IntoIter<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { vec: self, next: 0 }
    }
}

impl<A, B, N1, N2> PartialEq<Vec<B, N2>> for Vec<A, N1>
where
    N1: ArrayLength<A>,
    N2: ArrayLength<B>,
    A: PartialEq<B>,
{
    fn eq(&self, other: &Vec<B, N2>) -> bool {
        <[A]>::eq(self, &**other)
    }
}

macro_rules! eq {
    ($Lhs:ty, $Rhs:ty) => {
        impl<'a, 'b, A, B, N> PartialEq<$Rhs> for $Lhs
        where
            A: PartialEq<B>,
            N: ArrayLength<A>,
        {
            fn eq(&self, other: &$Rhs) -> bool {
                <[A]>::eq(self, &other[..])
            }
        }
    };
}

eq!(Vec<A, N>, [B]);
eq!(Vec<A, N>, &'a [B]);
eq!(Vec<A, N>, &'a mut [B]);

macro_rules! array {
    ($($N:expr),+) => {
        $(
            eq!(Vec<A, N>, [B; $N]);
            eq!(Vec<A, N>, &'a [B; $N]);
        )+
    }
}

array!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48,
    49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64
);

impl<T, N> Eq for Vec<T, N>
where
    N: ArrayLength<T>,
    T: Eq,
{
}

impl<T, N> ops::Deref for Vec<T, N>
where
    N: ArrayLength<T>,
{
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.0.as_slice()
    }
}

impl<T, N> ops::DerefMut for Vec<T, N>
where
    N: ArrayLength<T>,
{
    fn deref_mut(&mut self) -> &mut [T] {
        self.0.as_mut_slice()
    }
}

impl<T, N> AsRef<Vec<T, N>> for Vec<T, N>
where
    N: ArrayLength<T>,
{
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<T, N> AsMut<Vec<T, N>> for Vec<T, N>
where
    N: ArrayLength<T>,
{
    #[inline]
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<T, N> AsRef<[T]> for Vec<T, N>
where
    N: ArrayLength<T>,
{
    #[inline]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T, N> AsMut<[T]> for Vec<T, N>
where
    N: ArrayLength<T>,
{
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::{consts::*, Vec};
    use core::fmt::Write;

    #[test]
    fn static_new() {
        static mut _V: Vec<i32, U4> = Vec(crate::i::Vec::new());
    }

    macro_rules! droppable {
        () => {
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
        };
    }

    #[test]
    fn drop() {
        droppable!();

        {
            let mut v: Vec<Droppable, U2> = Vec::new();
            v.push(Droppable::new()).ok().unwrap();
            v.push(Droppable::new()).ok().unwrap();
            v.pop().unwrap();
        }

        assert_eq!(unsafe { COUNT }, 0);

        {
            let mut v: Vec<Droppable, U2> = Vec::new();
            v.push(Droppable::new()).ok().unwrap();
            v.push(Droppable::new()).ok().unwrap();
        }

        assert_eq!(unsafe { COUNT }, 0);
    }

    #[test]
    fn eq() {
        let mut xs: Vec<i32, U4> = Vec::new();
        let mut ys: Vec<i32, U8> = Vec::new();

        assert_eq!(xs, ys);

        xs.push(1).unwrap();
        ys.push(1).unwrap();

        assert_eq!(xs, ys);
    }

    #[test]
    fn full() {
        let mut v: Vec<i32, U4> = Vec::new();

        v.push(0).unwrap();
        v.push(1).unwrap();
        v.push(2).unwrap();
        v.push(3).unwrap();

        assert!(v.push(4).is_err());
    }

    #[test]
    fn iter() {
        let mut v: Vec<i32, U4> = Vec::new();

        v.push(0).unwrap();
        v.push(1).unwrap();
        v.push(2).unwrap();
        v.push(3).unwrap();

        let mut items = v.iter();

        assert_eq!(items.next(), Some(&0));
        assert_eq!(items.next(), Some(&1));
        assert_eq!(items.next(), Some(&2));
        assert_eq!(items.next(), Some(&3));
        assert_eq!(items.next(), None);
    }

    #[test]
    fn iter_mut() {
        let mut v: Vec<i32, U4> = Vec::new();

        v.push(0).unwrap();
        v.push(1).unwrap();
        v.push(2).unwrap();
        v.push(3).unwrap();

        let mut items = v.iter_mut();

        assert_eq!(items.next(), Some(&mut 0));
        assert_eq!(items.next(), Some(&mut 1));
        assert_eq!(items.next(), Some(&mut 2));
        assert_eq!(items.next(), Some(&mut 3));
        assert_eq!(items.next(), None);
    }

    #[test]
    fn collect_from_iter() {
        let slice = &[1, 2, 3];
        let vec = slice.iter().cloned().collect::<Vec<_, U4>>();
        assert_eq!(vec, slice);
    }

    #[test]
    #[should_panic]
    fn collect_from_iter_overfull() {
        let slice = &[1, 2, 3];
        let _vec = slice.iter().cloned().collect::<Vec<_, U2>>();
    }

    #[test]
    fn iter_move() {
        let mut v: Vec<i32, U4> = Vec::new();
        v.push(0).unwrap();
        v.push(1).unwrap();
        v.push(2).unwrap();
        v.push(3).unwrap();

        let mut items = v.into_iter();

        assert_eq!(items.next(), Some(0));
        assert_eq!(items.next(), Some(1));
        assert_eq!(items.next(), Some(2));
        assert_eq!(items.next(), Some(3));
        assert_eq!(items.next(), None);
    }

    #[test]
    fn iter_move_drop() {
        droppable!();

        {
            let mut vec: Vec<Droppable, U2> = Vec::new();
            vec.push(Droppable::new()).ok().unwrap();
            vec.push(Droppable::new()).ok().unwrap();
            let mut items = vec.into_iter();
            // Move all
            let _ = items.next();
            let _ = items.next();
        }

        assert_eq!(unsafe { COUNT }, 0);

        {
            let mut vec: Vec<Droppable, U2> = Vec::new();
            vec.push(Droppable::new()).ok().unwrap();
            vec.push(Droppable::new()).ok().unwrap();
            let _items = vec.into_iter();
            // Move none
        }

        assert_eq!(unsafe { COUNT }, 0);

        {
            let mut vec: Vec<Droppable, U2> = Vec::new();
            vec.push(Droppable::new()).ok().unwrap();
            vec.push(Droppable::new()).ok().unwrap();
            let mut items = vec.into_iter();
            let _ = items.next(); // Move partly
        }

        assert_eq!(unsafe { COUNT }, 0);
    }

    #[test]
    fn push_and_pop() {
        let mut v: Vec<i32, U4> = Vec::new();
        assert_eq!(v.len(), 0);

        assert_eq!(v.pop(), None);
        assert_eq!(v.len(), 0);

        v.push(0).unwrap();
        assert_eq!(v.len(), 1);

        assert_eq!(v.pop(), Some(0));
        assert_eq!(v.len(), 0);

        assert_eq!(v.pop(), None);
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn resize_size_limit() {
        let mut v: Vec<u8, U4> = Vec::new();

        v.resize(0, 0).unwrap();
        v.resize(4, 0).unwrap();
        v.resize(5, 0).err().expect("full");
    }

    #[test]
    fn resize_length_cases() {
        let mut v: Vec<u8, U4> = Vec::new();

        assert_eq!(v.len(), 0);

        // Grow by 1
        v.resize(1, 0).unwrap();
        assert_eq!(v.len(), 1);

        // Grow by 2
        v.resize(3, 0).unwrap();
        assert_eq!(v.len(), 3);

        // Resize to current size
        v.resize(3, 0).unwrap();
        assert_eq!(v.len(), 3);

        // Shrink by 1
        v.resize(2, 0).unwrap();
        assert_eq!(v.len(), 2);

        // Shrink by 2
        v.resize(0, 0).unwrap();
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn resize_contents() {
        let mut v: Vec<u8, U4> = Vec::new();

        // New entries take supplied value when growing
        v.resize(1, 17).unwrap();
        assert_eq!(v[0], 17);

        // Old values aren't changed when growing
        v.resize(2, 18).unwrap();
        assert_eq!(v[0], 17);
        assert_eq!(v[1], 18);

        // Old values aren't changed when length unchanged
        v.resize(2, 0).unwrap();
        assert_eq!(v[0], 17);
        assert_eq!(v[1], 18);

        // Old values aren't changed when shrinking
        v.resize(1, 0).unwrap();
        assert_eq!(v[0], 17);
    }

    #[test]
    fn resize_default() {
        let mut v: Vec<u8, U4> = Vec::new();

        // resize_default is implemented using resize, so just check the
        // correct value is being written.
        v.resize_default(1).unwrap();
        assert_eq!(v[0], 0);
    }

    #[test]
    fn write() {
        let mut v: Vec<u8, U4> = Vec::new();
        write!(v, "{:x}", 1234).unwrap();
        assert_eq!(&v[..], b"4d2");
    }

    #[test]
    fn extend_from_slice() {
        let mut v: Vec<u8, U4> = Vec::new();
        assert_eq!(v.len(), 0);
        v.extend_from_slice(&[1, 2]).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v.as_slice(), &[1, 2]);
        v.extend_from_slice(&[3]).unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(v.as_slice(), &[1, 2, 3]);
        assert!(v.extend_from_slice(&[4, 5]).is_err());
        assert_eq!(v.len(), 3);
        assert_eq!(v.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn from_slice() {
        // Successful construction
        let v: Vec<u8, U4> = Vec::from_slice(&[1, 2, 3]).unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(v.as_slice(), &[1, 2, 3]);

        // Slice too large
        assert!(Vec::<u8, U2>::from_slice(&[1, 2, 3]).is_err());
    }

    #[test]
    fn starts_with() {
        let v: Vec<_, U8> = Vec::from_slice(b"ab").unwrap();
        assert!(v.starts_with(&[]));
        assert!(v.starts_with(b""));
        assert!(v.starts_with(b"a"));
        assert!(v.starts_with(b"ab"));
        assert!(!v.starts_with(b"abc"));
        assert!(!v.starts_with(b"ba"));
        assert!(!v.starts_with(b"b"));
    }

    #[test]
    fn ends_with() {
        let v: Vec<_, U8> = Vec::from_slice(b"ab").unwrap();
        assert!(v.ends_with(&[]));
        assert!(v.ends_with(b""));
        assert!(v.ends_with(b"b"));
        assert!(v.ends_with(b"ab"));
        assert!(!v.ends_with(b"abc"));
        assert!(!v.ends_with(b"ba"));
        assert!(!v.ends_with(b"a"));
    }
}
