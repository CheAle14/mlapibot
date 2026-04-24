use digest_io::IoWrapper;
use sha2::Digest;

pub struct Sha256Hasher {
    hash: IoWrapper<sha2::Sha256>,
}

impl Sha256Hasher {
    #[inline]
    pub fn oneshot(bytes: impl AsRef<[u8]>) -> Base64Hash {
        let mut hasher = Self::new();
        hasher.write_bytes(bytes);
        hasher.finish()
    }

    pub fn new() -> Self {
        Self {
            hash: IoWrapper(sha2::Sha256::new()),
        }
    }

    pub fn write_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        use std::io::Write;

        let bytes: &[u8] = bytes.as_ref();
        let _ = write!(self.hash, "{}", bytes.len());

        self.hash.0.update(bytes);
    }

    pub fn finish(self) -> Base64Hash {
        let output = self.hash.0.finalize();
        Base64Hash::new(&output)
    }
}

mod _hash {
    use base64ct::{Base64, Encoding};

    // 4 bytes for every 3 in the input.
    // 32 bytes in => 32/3 = 10.666 sets of six bits.
    // 10.66 * 4 => 42.66 => 43 bytes, plus one padding.
    const MAX_LEN: usize = 44;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Base64Hash {
        bytes: [u8; MAX_LEN],
    }

    impl Base64Hash {
        pub fn new(hash: &[u8]) -> Self {
            let mut bytes = [0; MAX_LEN];
            Base64::encode(&hash, &mut bytes).expect("32 -> 44 length");
            Self { bytes }
        }

        /// `hash` must contain 44 bytes, representing a base64 encoded 32-byte hash.
        pub fn from_string(hash: &str) -> Self {
            let bytes: [u8; MAX_LEN] = match hash.as_bytes().try_into() {
                Ok(h) => h,
                Err(err) => panic!(
                    "expected length {MAX_LEN}, was {} {err}: {hash:?}",
                    hash.len()
                ),
            };

            Self { bytes }
        }

        pub fn as_str(&self) -> &str {
            self.as_ref()
        }
    }

    impl AsRef<str> for Base64Hash {
        fn as_ref(&self) -> &str {
            unsafe { str::from_utf8_unchecked(&self.bytes) }
        }
    }

    impl std::fmt::Display for Base64Hash {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let string: &str = self.as_ref();
            string.fmt(f)
        }
    }
}

pub use _hash::Base64Hash;

#[cfg(test)]
mod tests {
    #[test]
    pub fn sha256_hasher() {
        let mut hasher = super::Sha256Hasher::new();
        hasher.write_bytes("hello world");
        let hash = hasher.finish();

        assert_eq!(
            hash.as_str(),
            "OFtUHqk7C1JSe9bsEG6iiX6c+u/ei1A6Pq1+75Yr5y4="
        );
    }
}
