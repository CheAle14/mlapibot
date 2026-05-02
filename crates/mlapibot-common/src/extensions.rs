pub trait StringOptionExt {
    fn map_str(&self) -> Option<&str>;
}

impl StringOptionExt for Option<String> {
    fn map_str(&self) -> Option<&str> {
        match self {
            Some(s) => Some(s.as_str()),
            None => None,
        }
    }
}
