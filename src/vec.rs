use core::{fmt, ops, ptr, slice};

use generic_array::{ArrayLength, GenericArray};

use __core::mem::{ManuallyDrop, Uninit};

use core::iter::FromIterator;

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
pub struct Vec<T, N>
where
    N: ArrayLength<T>,
{
    buffer: ManuallyDrop<Uninit<GenericArray<T, N>>>,
    len: usize,
}

impl<T, N> Vec<T, N>
where
    N: ArrayLength<T>,
{
    /* Constructors */
    const_fn!(
        /// Constructs a new, empty vector with a fixed capacity of `N`
        pub const fn new() -> Self {
            Vec {
                buffer: ManuallyDrop::new(unsafe { Uninit::new() }),
                len: 0,
            }
        }
    );

    /* Public API */
    /// Returns the maximum number of elements the vector can hold
    pub fn capacity(&self) -> usize {
        N::to_usize()
    }

    /// Clears the vector, removing all values.
    pub fn clear(&mut self) {
        self.truncate(0);
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
        if self.len() + other.len() > self.capacity() {
            // won't fit in the `Vec`; don't modify anything and return an error
            Err(())
        } else {
            for elem in other {
                self.push(elem.clone()).ok();
            }
            Ok(())
        }
    }

    /// Removes the last element from a vector and return it, or `None` if it's empty
    pub fn pop(&mut self) -> Option<T> {
        if self.len != 0 {
            Some(unsafe { self.pop_unchecked() })
        } else {
            None
        }
    }

    pub(crate) unsafe fn pop_unchecked(&mut self) -> T {
        debug_assert!(!self.is_empty());

        let buffer = self.buffer.as_slice();

        self.len -= 1;
        let item = ptr::read(buffer.get_unchecked(self.len));
        item
    }

    /// Appends an `item` to the back of the collection
    ///
    /// Returns back the `item` if the vector is full
    pub fn push(&mut self, item: T) -> Result<(), T> {
        if self.len < self.capacity() {
            unsafe { self.push_unchecked(item) }
            Ok(())
        } else {
            Err(item)
        }
    }

    pub(crate) unsafe fn push_unchecked(&mut self, item: T) {
        let buffer = self.buffer.as_mut_slice();

        // NOTE(ptr::write) the memory slot that we are about to write to is uninitialized. We
        // use `ptr::write` to avoid running `T`'s destructor on the uninitialized memory
        ptr::write(buffer.get_unchecked_mut(self.len), item);
        self.len += 1;
    }

    /// Shortens the vector, keeping the first `len` elements and dropping the rest.
    pub fn truncate(&mut self, len: usize) {
        unsafe {
            // drop any extra elements
            while len < self.len {
                // decrement len before the drop_in_place(), so a panic on Drop
                // doesn't re-drop the just-failed value.
                self.len -= 1;
                let len = self.len;
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

        if new_len > self.len {
            while self.len < new_len {
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
        assert!(index < self.len());
        unsafe { self.swap_remove_unchecked(index) }
    }

    pub(crate) unsafe fn swap_remove_unchecked(&mut self, index: usize) -> T {
        let length = self.len();
        debug_assert!(index < length);
        ptr::swap(
            self.get_unchecked_mut(index),
            self.get_unchecked_mut(length - 1),
        );
        self.pop_unchecked()
    }

    /* Private API */
    pub(crate) fn is_full(&self) -> bool {
        self.capacity() == self.len()
    }
}

impl<T, N> fmt::Debug for Vec<T, N>
where
    T: fmt::Debug,
    N: ArrayLength<T>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let slice: &[T] = &**self;
        slice.fmt(f)
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
        for elem in iter {
            self.push(elem).ok().unwrap()
        }
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

impl <T, N> Iterator for IntoIter<T, N>
where
    N: ArrayLength<T>,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next < self.vec.len() {
            let buffer = self.vec.buffer.as_slice();
            let item = unsafe {ptr::read(buffer.get_unchecked(self.next))};
            self.next += 1;
            Some(item)
        } else {
            None
        }
    }
}

impl <T, N> Drop for IntoIter<T, N>
where
    N: ArrayLength<T>,
{
    fn drop(&mut self) {
        unsafe {
            // Drop all the elements that have not been moved out of vec
            ptr::drop_in_place(&mut self.vec[self.next..]);
            // Prevent dropping of other elements
            self.vec.len = 0;
        }
    }
}

impl <T, N> IntoIterator for Vec<T, N>
where
    N: ArrayLength<T>,
{
    type Item = T;
    type IntoIter = IntoIter<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            vec: self,
            next: 0,
        }
    }
}

impl<A, B, N1, N2> PartialEq<Vec<B, N2>> for Vec<A, N1>
where
    N1: ArrayLength<A>,
    N2: ArrayLength<B>,
    A: PartialEq<B>,
{
    fn eq(&self, other: &Vec<B, N2>) -> bool {
        self[..] == other[..]
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
                self[..] == other[..]
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
    26, 27, 28, 29, 30, 31, 32
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
        let buffer = self.buffer.as_slice();
        // NOTE(unsafe) avoid bound checks in the slicing operation
        // &buffer[..self.len]
        unsafe { slice::from_raw_parts(buffer.as_ptr(), self.len) }
    }
}

impl<T, N> ops::DerefMut for Vec<T, N>
where
    N: ArrayLength<T>,
{
    fn deref_mut(&mut self) -> &mut [T] {
        let len = self.len();
        let buffer = self.buffer.as_mut_slice();

        // NOTE(unsafe) avoid bound checks in the slicing operation
        // &mut buffer[..len]
        unsafe { slice::from_raw_parts_mut(buffer.as_mut_ptr(), len) }
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
    use consts::*;
    use Vec;

    #[cfg(feature = "const-fn")]
    #[test]
    fn static_new() {
        static mut _V: Vec<i32, U4> = Vec::new();
    }

    macro_rules! droppable {
        () => (
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
        )
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
}
