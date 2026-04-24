use std::ops::Deref;

use chrono::{DateTime, TimeZone, Utc};

pub fn clamp(text: &str, len: usize) -> &str {
    if text.len() > len { &text[..len] } else { text }
}

#[allow(unused)]
pub fn into_timestamp(utc: f64) -> DateTime<Utc> {
    (Utc).timestamp_millis_opt((utc * 1000.0) as i64).unwrap()
}

/// Borrow or Owned type.
pub enum BoO<'a, T> {
    Borrow(&'a T),
    Owned(T),
}

impl<'a, T> Deref for BoO<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            BoO::Borrow(b) => b,
            BoO::Owned(o) => o,
        }
    }
}
