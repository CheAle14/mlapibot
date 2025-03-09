use chrono::{DateTime, TimeZone, Utc};

pub fn clamp(text: &str, len: usize) -> &str {
    if text.len() > len { &text[..len] } else { text }
}

#[cfg(feature = "true-run")]
pub const fn is_debug() -> bool {
    false
}

#[cfg(not(feature = "true-run"))]
pub const fn is_debug() -> bool {
    true
}

pub fn into_timestamp(utc: f64) -> DateTime<Utc> {
    (Utc).timestamp_millis_opt((utc * 1000.0) as i64).unwrap()
}
