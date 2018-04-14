use core::borrow::Borrow;
use core::marker::Unsize;
use core::{mem, ops, slice};

use {BufferFullError, Vec};

/// A map / dictionary backed by an array that performs lookups via linear search
///
/// Note that as this map doesn't use hashing so most operations are **O(N)** instead of O(1)
pub struct LinearMap<KEY, VALUE, ARRAY>
where
    ARRAY: Unsize<[(KEY, VALUE)]>,
    KEY: Eq,
{
    buffer: Vec<(KEY, VALUE), ARRAY>,
}

impl<K, V, A> LinearMap<K, V, A>
where
    A: Unsize<[(K, V)]>,
    K: Eq,
{
    /// Creates an empty `LinearMap`
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<&str, isize, [_; 8]> = LinearMap::new();
    /// ```
    pub const fn new() -> Self {
        LinearMap { buffer: Vec::new() }
    }

    /// Returns the number of elements that the map can hold
    ///
    /// Computes in **O(1)** time
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<&str, isize, [_; 8]> = LinearMap::new();
    /// assert_eq!(map.capacity(), 8);
    /// ```
    pub fn capacity(&mut self) -> usize {
        self.buffer.capacity()
    }

    /// Clears the map, removing all key-value pairs
    ///
    /// Computes in **O(1)** time
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// map.insert(1, "a").unwrap();
    /// map.clear();
    /// assert!(map.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.buffer.clear()
    }

    /// Returns true if the map contains a value for the specified key.
    ///
    /// Computes in **O(N)** time
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// map.insert(1, "a").unwrap();
    /// assert_eq!(map.contains_key(&1), true);
    /// assert_eq!(map.contains_key(&2), false);
    /// ```
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Returns a reference to the value corresponding to the key
    ///
    /// Computes in **O(N)** time
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// map.insert(1, "a").unwrap();
    /// assert_eq!(map.get(&1), Some(&"a"));
    /// assert_eq!(map.get(&2), None);
    /// ```
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        self.iter()
            .find(|&(k, _)| k.borrow() == key)
            .map(|(_, v)| v)
    }

    /// Returns a mutable reference to the value corresponding to the key
    ///
    /// Computes in **O(N)** time
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// map.insert(1, "a").unwrap();
    /// if let Some(x) = map.get_mut(&1) {
    ///     *x = "b";
    /// }
    /// assert_eq!(map[&1], "b");
    /// ```
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        self.iter_mut()
            .find(|&(k, _)| k.borrow() == key)
            .map(|(_, v)| v)
    }

    /// Returns the number of elements in this map
    ///
    /// Computes in **O(1)** time
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut a: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// assert_eq!(a.len(), 0);
    /// a.insert(1, "a").unwrap();
    /// assert_eq!(a.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old value is returned.
    ///
    /// Computes in **O(N)** time
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// assert_eq!(map.insert(37, "a").unwrap(), None);
    /// assert_eq!(map.is_empty(), false);
    ///
    /// map.insert(37, "b").unwrap();
    /// assert_eq!(map.insert(37, "c").unwrap(), Some("b"));
    /// assert_eq!(map[&37], "c");
    /// ```
    pub fn insert(&mut self, key: K, mut value: V) -> Result<Option<V>, BufferFullError<(K, V)>> {
        if let Some((_, v)) = self.iter_mut().find(|&(k, _)| *k == key) {
            mem::swap(v, &mut value);
            return Ok(Some(value));
        }

        self.buffer.push((key, value))?;
        Ok(None)
    }

    /// Returns true if the map contains no elements
    ///
    /// Computes in **O(1)** time
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut a: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// assert!(a.is_empty());
    /// a.insert(1, "a").unwrap();
    /// assert!(!a.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// map.insert("a", 1).unwrap();
    /// map.insert("b", 2).unwrap();
    /// map.insert("c", 3).unwrap();
    ///
    /// for (key, val) in map.iter() {
    ///     println!("key: {} val: {}", key, val);
    /// }
    /// ```
    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            iter: self.buffer.iter(),
        }
    }

    /// An iterator visiting all key-value pairs in arbitrary order, with mutable references to the
    /// values
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// map.insert("a", 1).unwrap();
    /// map.insert("b", 2).unwrap();
    /// map.insert("c", 3).unwrap();
    ///
    /// // Update all values
    /// for (_, val) in map.iter_mut() {
    ///     *val = 2;
    /// }
    ///
    /// for (key, val) in &map {
    ///     println!("key: {} val: {}", key, val);
    /// }
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        IterMut {
            iter: self.buffer.iter_mut(),
        }
    }

    /// An iterator visiting all keys in arbitrary order
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// map.insert("a", 1).unwrap();
    /// map.insert("b", 2).unwrap();
    /// map.insert("c", 3).unwrap();
    ///
    /// for key in map.keys() {
    ///     println!("{}", key);
    /// }
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(k, _)| k)
    }

    /// Removes a key from the map, returning the value at the key if the key was previously in the
    /// map
    ///
    /// Computes in **O(N)** time
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// map.insert(1, "a").unwrap();
    /// assert_eq!(map.remove(&1), Some("a"));
    /// assert_eq!(map.remove(&1), None);
    /// ```
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        let idx = self.keys()
            .enumerate()
            .find(|&(_, k)| k.borrow() == key)
            .map(|(idx, _)| idx);

        idx.map(|idx| self.buffer.swap_remove(idx).1)
    }

    /// An iterator visiting all values in arbitrary order
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// map.insert("a", 1).unwrap();
    /// map.insert("b", 2).unwrap();
    /// map.insert("c", 3).unwrap();
    ///
    /// for val in map.values() {
    ///     println!("{}", val);
    /// }
    /// ```
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, v)| v)
    }

    /// An iterator visiting all values mutably in arbitrary order
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::LinearMap;
    ///
    /// let mut map: LinearMap<_, _, [_; 8]> = LinearMap::new();
    /// map.insert("a", 1).unwrap();
    /// map.insert("b", 2).unwrap();
    /// map.insert("c", 3).unwrap();
    ///
    /// for val in map.values_mut() {
    ///     *val += 10;
    /// }
    ///
    /// for val in map.values() {
    ///     println!("{}", val);
    /// }
    /// ```
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.iter_mut().map(|(_, v)| v)
    }
}

impl<'a, K, V, A, Q> ops::Index<&'a Q> for LinearMap<K, V, A>
where
    A: Unsize<[(K, V)]>,
    K: Borrow<Q> + Eq,
    Q: Eq + ?Sized,
    V: 'a,
{
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("no entry found for key")
    }
}

impl<'a, K, V, A, Q> ops::IndexMut<&'a Q> for LinearMap<K, V, A>
where
    A: Unsize<[(K, V)]>,
    K: Borrow<Q> + Eq,
    Q: Eq + ?Sized,
    V: 'a,
{
    fn index_mut(&mut self, key: &Q) -> &mut V {
        self.get_mut(key).expect("no entry found for key")
    }
}

impl<'a, K, V, A> IntoIterator for &'a LinearMap<K, V, A>
where
    A: Unsize<[(K, V)]>,
    K: Eq,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

pub struct Iter<'a, K, V>
where
    K: 'a,
    V: 'a,
{
    iter: slice::Iter<'a, (K, V)>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: 'a,
    V: 'a,
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|&(ref k, ref v)| (k, v))
    }
}

pub struct IterMut<'a, K, V>
where
    K: 'a,
    V: 'a,
{
    iter: slice::IterMut<'a, (K, V)>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V>
where
    K: 'a,
    V: 'a,
{
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|&mut (ref k, ref mut v)| (k, v))
    }
}
