use std::num::NonZeroU32;

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq)]
pub struct Url {
    // "https://example.com/some/path?query=wow"
    text: String,

    scheme_end: u32,
    // domain start is "://" after scheme end
    domain_end: u32,
    path_end: Option<NonZeroU32>,
    query_end: Option<NonZeroU32>,
}

impl Url {
    const DOMAIN_START_OFFSET: u32 = "://".len() as u32;

    pub fn parse(text: &str) -> Result<Self, ()> {
        let (scheme, rest) = text.split_once("://").ok_or(())?;
        let scheme_end = scheme.len() as u32;

        let (domain_end, path_end, query_end) = if let Some((domain, path)) = rest.split_once('/') {
            let domain_end = scheme_end + Self::DOMAIN_START_OFFSET + domain.len() as u32;
            if let Some((path, query)) = path.split_once('?') {
                let path = domain_end + 1 + path.len() as u32;
                (domain_end, Some(path), Some(path + 1 + query.len() as u32))
            } else {
                (domain_end, Some(domain_end + 1 + path.len() as u32), None)
            }
        } else if let Some((domain, query)) = rest.split_once('?') {
            let domain_end = scheme_end + Self::DOMAIN_START_OFFSET + domain.len() as u32;
            (domain_end, None, Some(domain_end + 1 + query.len() as u32))
        } else {
            let domain_end = scheme_end + Self::DOMAIN_START_OFFSET + rest.len() as u32;
            (domain_end, None, None)
        };

        let path_end = path_end.map(|i| unsafe {
            // the scheme separator ("://") comes before this and must always be present, so the position
            // here is always at least 3, which is clearly above zero
            NonZeroU32::new_unchecked(i)
        });

        let query_end = query_end.map(|i| unsafe {
            // same reasoning as above
            NonZeroU32::new_unchecked(i)
        });

        Ok(Self {
            text: text.to_owned(),
            scheme_end,
            domain_end,
            path_end: path_end,
            query_end,
        })
    }

    pub fn scheme(&self) -> &str {
        &self.text[..self.scheme_end as usize]
    }

    pub fn domain(&self) -> &str {
        let start = self.scheme_end + Self::DOMAIN_START_OFFSET;
        &self.text[start as usize..self.domain_end as usize]
    }

    pub fn set_domain(&mut self, domain: &str) {
        let new = format!(
            "{}://{}{}{}",
            self.scheme(),
            domain,
            self.path(),
            self.query()
        );
        *self = Url::parse(&new).unwrap();
    }

    pub fn path(&self) -> &str {
        if let Some(path_end) = self.path_end {
            let start = self.domain_end;
            &self.text[start as usize..path_end.get() as usize]
        } else {
            ""
        }
    }

    pub fn set_path(&mut self, path: &str) {
        let new = format!(
            "{}://{}{}{}",
            self.scheme(),
            self.domain(),
            path,
            self.query()
        );
        *self = Url::parse(&new).unwrap();
    }

    pub fn query(&self) -> &str {
        match (self.path_end, self.query_end) {
            (None, None) | (Some(_), None) => "",
            (None, Some(end)) => &self.text[self.domain_end as usize..end.get() as usize],
            (Some(start), Some(end)) => &self.text[start.get() as usize..end.get() as usize],
        }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl<'de> Deserialize<'de> for Url {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let s = String::deserialize(deserializer)?;
        Url::parse(&s).map_err(|()| D::Error::custom("invalid url"))
    }
}

impl<'a> From<&'a str> for Url {
    fn from(value: &'a str) -> Self {
        Url::parse(value).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_full() {
        let url = Url::parse("https://example.com/some/path?query=wow").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.domain(), "example.com");
        assert_eq!(url.path(), "/some/path");
        assert_eq!(url.query(), "?query=wow");
    }
    #[test]
    pub fn test_path_no_query() {
        let url = Url::parse("https://example.com/some/path").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.domain(), "example.com");
        assert_eq!(url.path(), "/some/path");
        assert_eq!(url.query(), "");
    }
    #[test]
    pub fn test_query_no_path() {
        let url = Url::parse("https://example.com?hello=world").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.domain(), "example.com");
        assert_eq!(url.path(), "");
        assert_eq!(url.query(), "?hello=world");
    }
    #[test]
    pub fn test_no_query_no_path() {
        let url = Url::parse("https://example.com").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.domain(), "example.com");
        assert_eq!(url.path(), "");
        assert_eq!(url.query(), "");
    }

    #[test]
    pub fn test_modify() {
        let mut url = Url::parse("https://example.com/some/path?query=wow").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.domain(), "example.com");
        assert_eq!(url.path(), "/some/path");
        assert_eq!(url.query(), "?query=wow");

        url.set_domain("other.domain.com");
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.domain(), "other.domain.com");
        assert_eq!(url.path(), "/some/path");
        assert_eq!(url.query(), "?query=wow");

        url.set_path("/a/new/path/goes/here");
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.domain(), "other.domain.com");
        assert_eq!(url.path(), "/a/new/path/goes/here");
        assert_eq!(url.query(), "?query=wow");
    }

    #[test]
    pub fn correctly_fails() {
        assert_eq!(Url::parse(""), Err(()));
        assert_eq!(Url::parse("something"), Err(()));
        assert_eq!(Url::parse("example.com/wow?hello=there"), Err(()));
    }
}
