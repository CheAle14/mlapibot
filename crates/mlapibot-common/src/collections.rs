use std::borrow::Borrow;

pub struct OrderedSet<T> {
    inner: Vec<T>,
}

impl<T> FromIterator<T> for OrderedSet<T>
where
    T: Ord,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (lower, upper) = iter.size_hint();
        let size = upper.unwrap_or(lower);

        let mut this = Self {
            inner: Vec::with_capacity(size),
        };

        for item in iter {
            this.add(item);
        }

        this
    }
}

impl<T> OrderedSet<T> {
    pub fn new_assert_ordered(inner: Vec<T>) -> Self {
        Self { inner }
    }

    /// Returns `true` if item was added, `false` if already present in the set.
    pub fn add(&mut self, value: T) -> bool
    where
        T: Ord,
    {
        match self.inner.binary_search(&value) {
            Ok(_) => false,
            Err(idx) => {
                self.inner.insert(idx, value);
                true
            }
        }
    }

    fn find_index<Q>(&self, query: &Q) -> Result<usize, usize>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner
            .binary_search_by(|value| value.borrow().cmp(query))
    }

    pub fn contains<Q>(&self, query: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.find_index(query).is_ok()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use crate::collections::OrderedSet;

    #[test]
    fn ordered_set_ints() {
        let items = [5, 10, 2, 0, 3, -5, 9, 1, 2, 10];

        let set: OrderedSet<i32> = items.into_iter().collect();

        assert_eq!(set.len(), 8);

        assert!(set.contains(&5));
        assert!(set.contains(&10));
        assert!(set.contains(&2));
        assert!(set.contains(&0));
        assert!(set.contains(&3));
        assert!(set.contains(&-5));
        assert!(set.contains(&9));
        assert!(set.contains(&1));

        assert!(!set.contains(&99));
        assert!(!set.contains(&-1));
    }

    #[test]
    fn ordered_set_strings() {
        let texts = ["a", "b", "z", "e", "dd", "df", "dd", "hi", "lo"];

        let set: OrderedSet<String> = texts.into_iter().map(ToOwned::to_owned).collect();

        assert_eq!(set.len(), 8);

        assert!(set.contains("dd"));

        assert!(!set.contains("hello world"));
    }
}
