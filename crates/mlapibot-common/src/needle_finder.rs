pub struct NeedleFinder<'needle, 'haystack> {
    needle: &'needle str,
    haystack: &'haystack str,
    offset: usize,
}

impl<'n, 'h> NeedleFinder<'n, 'h> {
    pub fn new(needle: &'n str, haystack: &'h str) -> Self {
        Self {
            needle,
            haystack,
            offset: 0,
        }
    }
}

impl<'n, 'h> Iterator for NeedleFinder<'n, 'h> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.haystack.find(self.needle)?;
        self.haystack = &self.haystack[(next + 1)..];

        let idx = self.offset + next;

        self.offset += next + 1;

        Some(idx)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn test_needler() {
        let mut finder = super::NeedleFinder::new("hello", "hello world world hello there");

        println!("{}", &"hello world world hello there"[17..]);

        assert_eq!(finder.next(), Some(0));
        assert_eq!(finder.next(), Some(18));
        assert_eq!(finder.next(), None);
    }
}
