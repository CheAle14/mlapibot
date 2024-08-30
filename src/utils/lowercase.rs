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

impl std::fmt::Display for LowercaseString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> Deserialize<'de> for LowercaseString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match String::deserialize(deserializer) {
            Ok(mut s) => Ok(Self::new(s)),
            Err(e) => Err(e),
        }
    }
}
