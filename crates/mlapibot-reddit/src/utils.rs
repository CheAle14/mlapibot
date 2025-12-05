use base64ct::{Base64, Encoding};
use chrono::{DateTime, TimeZone, Utc};
use sha2::Digest;

pub fn clamp(text: &str, len: usize) -> &str {
    if text.len() > len { &text[..len] } else { text }
}

pub fn into_timestamp(utc: f64) -> DateTime<Utc> {
    (Utc).timestamp_millis_opt((utc * 1000.0) as i64).unwrap()
}

pub struct Sha256Hasher {
    hash: sha2::Sha256,
}

impl Sha256Hasher {
    #[inline]
    pub fn oneshot(bytes: impl AsRef<[u8]>) -> String {
        let mut hasher = Self::new();
        hasher.write_bytes(bytes);
        hasher.finish()
    }

    pub fn new() -> Self {
        Self {
            hash: sha2::Sha256::new(),
        }
    }

    pub fn write_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        use std::io::Write;

        let bytes: &[u8] = bytes.as_ref();

        let _ = write!(self.hash, "{}", bytes.len());

        self.hash.update(bytes);
    }

    pub fn finish(self) -> String {
        let output = self.hash.finalize();
        Base64::encode_string(&output)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn sha256_hasher() {
        let mut hasher = super::Sha256Hasher::new();
        hasher.write_bytes("hello world");
        let hash = hasher.finish();

        assert_eq!(hash, "OFtUHqk7C1JSe9bsEG6iiX6c+u/ei1A6Pq1+75Yr5y4=");
    }
}
