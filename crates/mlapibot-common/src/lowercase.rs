use std::{borrow::Borrow, cmp::Ordering, collections::HashMap, hash::Hash};

use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct LowercaseString(String);

impl LowercaseString {
    pub fn new(text: impl Into<String>) -> Self {
        let mut text: String = text.into();
        text.make_ascii_lowercase();
        Self(text)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'_ str> for LowercaseString {
    fn from(value: &'_ str) -> Self {
        Self::new(value)
    }
}

impl From<String> for LowercaseString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for LowercaseString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> serde::Deserialize<'de> for LowercaseString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match String::deserialize(deserializer) {
            Ok(s) => Ok(Self::new(s)),
            Err(e) => Err(e),
        }
    }
}

impl PartialEq<String> for LowercaseString {
    fn eq(&self, other: &String) -> bool {
        self.0.eq_ignore_ascii_case(&other)
    }
}

impl PartialEq<str> for LowercaseString {
    fn eq(&self, other: &str) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

impl<'a> Into<String> for &'a LowercaseString {
    fn into(self) -> String {
        self.0.clone()
    }
}

impl Borrow<str> for LowercaseString {
    fn borrow(&self) -> &str {
        self.0.as_str()
    }
}

/// Conceptually, a HashMap<LowercaseString, V>.
///
/// The case for the keys is ignored/normalised, so "hello" and "HELLO" both
/// refer to the same entry.
#[derive(Debug)]
pub struct LowercaseHashMap<V> {
    items: Vec<(LowercaseString, V)>,
}

fn ignore_ascii_case_order(needle: &str, possible: &str) -> Ordering {
    match needle.len().cmp(&possible.len()) {
        Ordering::Equal => (),
        other => return other,
    };

    for (needle, possible) in needle.chars().zip(possible.chars()) {
        match needle
            .to_ascii_lowercase()
            .cmp(&possible.to_ascii_lowercase())
        {
            Ordering::Equal => (),
            other => return other,
        }
    }

    Ordering::Equal
}

impl<V> LowercaseHashMap<V> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn insert<K>(&mut self, key: K, value: V) -> Option<V>
    where
        K: Borrow<str>,
        LowercaseString: From<K>,
    {
        let text: &str = key.borrow();

        match self
            .items
            .binary_search_by(|(possible, _)| ignore_ascii_case_order(text, possible.as_str()))
        {
            Ok(idx) => {
                let mut value = value;
                std::mem::swap(&mut value, &mut self.items[idx].1);
                Some(value)
            }
            Err(idx) => {
                self.items.insert(idx, (LowercaseString::from(key), value));
                None
            }
        }
    }

    pub fn get<K>(&self, key: &K) -> Option<&V>
    where
        K: Borrow<str> + ?Sized,
    {
        let text: &str = key.borrow();
        match self
            .items
            .binary_search_by(|(possible, _)| ignore_ascii_case_order(text, possible.as_str()))
        {
            Ok(idx) => self.items.get(idx).map(|v| &v.1),
            Err(_) => None,
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &LowercaseString> {
        self.items.iter().map(|v| &v.0)
    }
}

impl<V> From<HashMap<LowercaseString, V>> for LowercaseHashMap<V> {
    fn from(mapped: HashMap<LowercaseString, V>) -> Self {
        let mut this = Self::with_capacity(mapped.len());

        for (key, value) in mapped {
            this.insert(key, value);
        }

        this
    }
}

impl<'de, V> Deserialize<'de> for LowercaseHashMap<V>
where
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        HashMap::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{LowercaseHashMap, LowercaseString};

    #[test]
    pub fn lowercase_hash_map_from() {
        let mut hashmap = HashMap::new();
        hashmap.insert(LowercaseString::from("BCD"), 1);
        hashmap.insert(LowercaseString::from("aBc"), 2);
        hashmap.insert(LowercaseString::from("Cde"), 3);
        hashmap.insert(LowercaseString::from("abc"), 4);
        hashmap.insert(LowercaseString::from("bcd"), 5);
        assert_eq!(hashmap.len(), 3);

        let map = LowercaseHashMap::from(hashmap);
        assert_eq!(map.len(), 3);

        assert_eq!(map.get("abc"), Some(&4));
        assert_eq!(map.get("bcd"), Some(&5));
        assert_eq!(map.get("cde"), Some(&3));
    }

    #[test]
    pub fn lowercase_hash_map() {
        let mut map = LowercaseHashMap::new();
        map.insert("hello", 5);
        map.insert("Hello", 10);
        map.insert("wOrlD", 20);
        map.insert("world", 15);

        assert_eq!(map.len(), 2);

        assert_eq!(map.get("hello"), Some(&10));
        assert_eq!(map.get("heLLo"), Some(&10));

        assert_eq!(map.get("world"), Some(&15));
        assert_eq!(map.get("WORLD"), Some(&15));
    }
}
