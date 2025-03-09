use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DetectedWord {
    /// whether this word was part of the threshold triggering phrase
    pub matched: bool,
}

#[derive(Clone)]
pub struct DetectedItem {
    /// the words that were present or triggered
    pub words: HashMap<usize, DetectedWord>,
    pub score: f32,
}

impl std::fmt::Debug for DetectedItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (min, max) = self.min_max_word_indexes();
        f.debug_struct("DetectedItem")
            .field("words", &self.words)
            .field("score", &self.score)
            .field("min_idx", &min)
            .field("max_idx", &max)
            .finish()
    }
}

impl DetectedItem {
    pub fn new(score: f32) -> Self {
        Self {
            words: HashMap::new(),
            score,
        }
    }

    pub fn mark_match(&mut self, index: usize) {
        self.words
            .entry(index)
            .and_modify(|w| w.matched = true)
            .or_insert_with(|| DetectedWord { matched: true });
    }

    pub fn write_markdown<W: std::fmt::Write>(
        &self,
        text: &[impl AsRef<str>],
        output: &mut W,
    ) -> std::fmt::Result {
        let last_idx = text.len() - 1;
        for (idx, word) in text.iter().enumerate() {
            let word = word.as_ref();
            if let Some(_) = self.words.get(&idx) {
                write!(output, "**{word}**")?;
            } else {
                write!(output, "{word}")?;
            }
            if idx < last_idx {
                write!(output, " ")?;
            }
        }

        Ok(())
    }

    pub fn min_max_word_indexes(&self) -> (usize, usize) {
        let mut max = usize::MIN;
        let mut min = usize::MAX;

        for key in self.words.keys() {
            max = (*key).max(max);
            min = (*key).min(min);
        }

        (min, max)
    }

    pub fn range(&self) -> usize {
        let (min, max) = self.min_max_word_indexes();
        max - min
    }
}

impl std::ops::AddAssign for DetectedItem {
    fn add_assign(&mut self, rhs: Self) {
        self.words.extend(rhs.words);
        self.score += rhs.score;
    }
}

impl PartialEq for DetectedItem {
    fn eq(&self, other: &Self) -> bool {
        self.words == other.words && self.score == other.score
    }
}

impl Eq for DetectedItem {}

impl std::hash::Hash for DetectedItem {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut ordered = std::collections::BTreeSet::new();
        for word in self.words.keys() {
            ordered.insert(*word);
        }
        ordered.hash(state);
        state.write_u32(self.score as u32);
    }
}

impl PartialOrd for DetectedItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DetectedItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match other.score.total_cmp(&self.score) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        self.range().cmp(&other.range())
    }
}
