use ord_many::max_many;
use serde::Deserialize;

use crate::analysis::Detection;

use super::DetectedItem;

const THRESHOLD: f32 = 0.8;
const MATCH: i32 = 5;
const MISMATCH: i32 = -MATCH;
const BOUNDED_DISTANCE: usize = (MATCH - MISMATCH) as usize;
const INDEL: i32 = MISMATCH;

#[inline(always)]
fn string_similiarity(a: &str, b: &str) -> f32 {
    strsim::normalized_damerau_levenshtein(a, b) as f32
}

#[inline(always)]
fn match_or_mismatch(top: &str, side: &str) -> i32 {
    let perc = string_similiarity(top, side);
    let interpolate = perc * (BOUNDED_DISTANCE as f32);
    let match_mismatch = MISMATCH + (interpolate.ceil() as i32);
    match_mismatch
}

pub enum AlignmentKind {
    Matches { i: usize, j: usize, ratio: i32 },
    MissingI { j: usize },
    MissingJ { i: usize },
}

impl AlignmentKind {
    pub fn is_match(&self) -> bool {
        match self {
            Self::Matches { ratio, .. } => *ratio >= 4,
            _ => false,
        }
    }
    pub fn get_i<'a>(&self, text: &[&'a str]) -> Option<&'a str> {
        match self {
            Self::Matches { i, .. } => Some(text[*i]),
            Self::MissingI { .. } => None,
            Self::MissingJ { i } => Some(text[*i]),
        }
    }

    pub fn get_j<'b>(&self, text: &[&'b str]) -> Option<&'b str> {
        match self {
            Self::Matches { j, .. } => Some(text[*j]),
            Self::MissingI { j } => Some(text[*j]),
            Self::MissingJ { .. } => None,
        }
    }
}

type Alignment = Vec<AlignmentKind>;

pub fn score(arr: &Alignment, i: &[&str], j: &[&str], debug: bool) -> f32 {
    let mut first_match = arr.len();
    let mut last_match = 0;
    let mut mapped: Vec<(Option<&str>, Option<&str>)> = Vec::with_capacity(arr.len());

    for (idx, item) in arr.iter().enumerate() {
        if item.is_match() {
            first_match = std::cmp::min(first_match, idx);
            last_match = std::cmp::max(last_match, idx);
        }
        mapped.push((item.get_i(i), item.get_j(j)));
    }

    if first_match > last_match {
        return 0.0;
    }

    let selected = &mapped[first_match..=last_match];

    if debug {
        let words: String = selected
            .iter()
            .map(|(i, j)| format!("{}/{},", i.unwrap_or("?"), j.unwrap_or("?")))
            .collect();
        println!("    Matches between {first_match} -> {last_match}: {words:?}");
    }

    #[derive(Debug)]
    enum RunKind {
        Match(usize, usize),
        NonMatch(usize, usize),
    }

    impl RunKind {
        pub fn bounds(&self) -> (usize, usize) {
            match self {
                RunKind::Match(s, e) | RunKind::NonMatch(s, e) => (*s, *e),
            }
        }
    }

    let mut all_runs = Vec::new();
    let mut longest_match_run: Option<(usize, usize)> = None;

    let mut match_start = Some(0);
    let mut nonmatch_start = None;

    let mut current = 0;
    while current < selected.len() {
        let here = &selected[current];
        if let (Some(_), Some(_)) = here {
            // match
            if let Some(start) = nonmatch_start {
                all_runs.push(RunKind::NonMatch(start, current));

                nonmatch_start = None;
                match_start = Some(current);
            }
        } else {
            // not match
            if let Some(start) = match_start {
                let size = start.abs_diff(current);
                if longest_match_run.is_none() || longest_match_run.as_ref().unwrap().0 < size {
                    longest_match_run = Some((size, all_runs.len()));
                }
                all_runs.push(RunKind::Match(start, current));

                match_start = None;
                nonmatch_start = Some(current);
            }
        }
        current += 1;
    }
    if let Some(start) = match_start {
        let size = start.abs_diff(current);
        if longest_match_run.is_none() || longest_match_run.as_ref().unwrap().0 < size {
            longest_match_run = Some((size, all_runs.len()));
        }
        all_runs.push(RunKind::Match(start, current));
    } else if let Some(start) = nonmatch_start {
        all_runs.push(RunKind::NonMatch(start, current));
    }

    let (length, run_idx) = longest_match_run.unwrap();
    if debug {
        println!("    Longest matching run is {length}: {all_runs:?}");
    }
    let (mut selected_start, mut selected_end) = &all_runs[run_idx].bounds();

    let mut current = run_idx.saturating_sub(1);
    let mut allowed_length = length;
    while current > 0 {
        match &all_runs[current] {
            RunKind::NonMatch(s, e) => {
                if allowed_length.saturating_sub(s.abs_diff(*e)) == 0 {
                    break;
                }
            }
            RunKind::Match(s, e) => {
                allowed_length += s.abs_diff(*e);
                selected_start = std::cmp::min(selected_start, *s);
            }
        }
        current -= 1;
    }

    let mut current = run_idx + 1;
    let mut allowed_length = length;
    while current < all_runs.len() {
        match &all_runs[current] {
            RunKind::NonMatch(s, e) => {
                if allowed_length.saturating_sub(s.abs_diff(*e)) == 0 {
                    break;
                }
            }
            RunKind::Match(s, e) => {
                allowed_length += s.abs_diff(*e);
                selected_end = std::cmp::max(selected_end, *e);
            }
        }
        current += 1;
    }

    if debug {
        println!("    Reselected as {selected_start} -> {selected_end}");
    }

    let selected = &selected[selected_start..selected_end];

    let total = i.len() as f32;
    let mut sum = 0.0;
    for (i, j) in selected {
        match (i, j) {
            (Some(i), Some(j)) => sum += string_similiarity(i, j),
            _ => (),
        }
    }

    if debug {
        println!("    {sum} out of {sum}");
    }

    sum / total
}

fn _pretty_print_alignment(al: &Alignment) {
    print!("[");
    for item in al {
        match item {
            AlignmentKind::Matches { i, j, ratio } => {
                if *ratio >= 4 {
                    print!("M({i}/{j})")
                } else {
                    print!("N({i}/{j})")
                }
            }
            AlignmentKind::MissingI { j } => print!("_?/{j}_"),
            AlignmentKind::MissingJ { i } => print!("_{i}/?_"),
        }
        print!(", ");
    }
    println!("]");
}

pub fn needleman_wunsch<'a, 'b>(a: &[&'a str], b: &[&'b str]) -> Alignment {
    let columns = a.len() + 1;
    let rows = b.len() + 1;
    let mut matrix = vec![vec![0; rows]; columns];
    matrix[0][0] = 0;
    for i in 1..columns {
        matrix[i][0] = matrix[i - 1][0] + INDEL;
    }
    for j in 1..rows {
        matrix[0][j] = matrix[0][j - 1] + INDEL;
    }

    for i in 1..columns {
        for j in 1..rows {
            let diagonal = matrix[i - 1][j - 1] + match_or_mismatch(b[j - 1], a[i - 1]);
            let indel_side = matrix[i - 1][j] + INDEL;
            let indel_top = matrix[i][j - 1] + INDEL;
            matrix[i][j] = max_many!(indel_top, indel_side, diagonal);
        }
    }

    let mut alignment = Vec::new();
    let mut i = a.len();
    let mut j = b.len();

    while i > 0 || j > 0 {
        if i > 0
            && j > 0
            && matrix[i][j] == (matrix[i - 1][j - 1] + match_or_mismatch(a[i - 1], b[j - 1]))
        {
            alignment.push(AlignmentKind::Matches {
                i: i - 1,
                j: j - 1,
                ratio: match_or_mismatch(a[i - 1], b[j - 1]),
            });
            i -= 1;
            j -= 1;
        } else if i > 0 && matrix[i][j] == (matrix[i - 1][j] + INDEL) {
            alignment.push(AlignmentKind::MissingJ { i: i - 1 });
            i -= 1;
        } else {
            alignment.push(AlignmentKind::MissingI { j: j - 1 });
            j -= 1;
        }
    }

    alignment.reverse();
    alignment
}

fn allowed_char(c: char) -> bool {
    match c {
        'a'..='z' => true,
        '0'..='9' => true,
        _ => false,
    }
}

pub type CleanedWords = Vec<String>;

pub fn clean_string(string: impl AsRef<str>) -> String {
    let mut r = string.as_ref().to_lowercase();
    r.retain(allowed_char);
    r
}

pub fn get_words(text: &str) -> CleanedWords {
    let mut words = Vec::new();
    for word in text.split_ascii_whitespace() {
        let word = clean_string(word);
        if word.len() > 0 {
            words.push(word);
        }
    }
    words
}

fn as_ref(words: &Vec<String>) -> Vec<&str> {
    words.iter().map(|s| s.as_str()).collect()
}

fn _pretty_print_matrix(twod: &Vec<Vec<i32>>) {
    let mut displays = Vec::new();
    let mut longest = 0;
    for row in twod {
        let mut display = Vec::new();
        for cell in row {
            let s = cell.to_string();
            longest = std::cmp::max(longest, s.len());
            display.push(s);
        }
        displays.push(display);
    }

    for row in displays {
        for cell in row {
            print!("{cell:^width$} ", width = longest)
        }
        print!("\n");
    }
}

#[derive(Debug, Deserialize)]
#[serde(from = "Vec<String>")]
pub struct WordMatcher(Vec<CleanedWords>);

impl From<Vec<String>> for WordMatcher {
    fn from(value: Vec<String>) -> Self {
        Self::new(value)
    }
}

impl WordMatcher {
    pub fn new(uncleaned: Vec<String>) -> Self {
        let mapped = uncleaned.iter().map(|s| get_words(&s)).collect();
        Self(mapped)
    }
    pub fn matches(&self, haystack: &[&str], debug: bool) -> Option<DetectedItem> {
        let mut best: Option<(f32, Alignment)> = None;
        for text in &self.0 {
            let words: Vec<_> = as_ref(text);
            if debug {
                println!("  Looking for {:?}", text.join(" "))
            }
            let alignment = needleman_wunsch(&words, haystack);
            let score = score(&alignment, &words, haystack, debug);
            if debug {
                println!("  Score: {score}");
            }
            if best.is_none() || best.as_ref().unwrap().0 < score {
                best = Some((score, alignment));
            }
        }

        best.filter(|(s, _)| *s >= THRESHOLD)
            .map(|(score, alignment)| {
                let mut item = DetectedItem::new(score);
                for entry in alignment.iter() {
                    match entry {
                        AlignmentKind::Matches { j, .. } => item.mark_match(*j),
                        AlignmentKind::MissingI { .. } => (),
                        AlignmentKind::MissingJ { .. } => (),
                    }
                }
                item
            })
    }

    pub fn any_matches(&self, ctx: &crate::context::Context) -> bool {
        for img in &ctx.images {
            let words = img.words();
            if self.matches(&words, ctx.debug).is_some() {
                return true;
            }
        }
        if let Some(title) = &ctx.title {
            let words = get_words(&title);
            if self.matches(&as_ref(&words), ctx.debug).is_some() {
                return true;
            }
        }
        if let Some(body) = &ctx.body {
            let words = get_words(&body);
            if self.matches(&as_ref(&words), ctx.debug).is_some() {
                return true;
            }
        }

        false
    }
}

#[derive(Debug, Deserialize)]
pub struct StrAnalzyer {
    ocr: Option<WordMatcher>,
    title: Option<WordMatcher>,
    body: Option<WordMatcher>,
}

impl StrAnalzyer {
    pub fn analyze(
        &self,
        context: &crate::context::Context,
    ) -> anyhow::Result<Option<super::Detection>> {
        let mut detection = Detection::new();
        if let Some(ocr) = &self.ocr {
            for (idx, image) in context.images.iter().enumerate() {
                let words = image.words();
                if context.debug {
                    println!("OCR Image {idx}:");
                }
                if let Some(result) = ocr.matches(&words, context.debug) {
                    detection.add_image(idx, result);
                }
            }
        }
        if let Some(title) = &self.title {
            if let Some(ctx) = &context.title {
                let words = get_words(&ctx);
                if let Some(value) = title.matches(&as_ref(&words), context.debug) {
                    detection.set_title(value);
                }
            }
        }
        if let Some(body) = &self.body {
            if let Some(ctx) = &context.body {
                let words = get_words(&ctx);
                if let Some(value) = body.matches(&as_ref(&words), context.debug) {
                    detection.set_body(value);
                }
            }
        }
        Ok(detection.finish())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::context::{Context, ContextKind};

    use super::*;

    #[test]
    pub fn string_sim() {
        assert!(string_similiarity("15x", "1pumngutjqwevvqweuyg7") < THRESHOLD);
        assert!(string_similiarity("get", "1pumngutjqwevvqweuyg7") < THRESHOLD);
    }

    #[test]
    pub fn works() {
        assert_eq!(match_or_mismatch("hello", "hello"), MATCH);
        assert_eq!(match_or_mismatch("abcdf", "hello"), MISMATCH);

        let phrase = "the quick brown fox jumps over the lazy dog";
        let analyzer = StrAnalzyer {
            ocr: None,
            title: Some(WordMatcher::new(vec![phrase.to_string()])),
            body: None,
        };

        let ctx = Context {
            kind: ContextKind::CliPath(PathBuf::new()),
            images: Vec::new(),
            title: Some(String::from("some other words like lots of words on either side but the phrase is still there in the picture somewhere the quick brown fox jumps over the lazy and even more words go on this side of the picture it is unbelievable that it is so long over here dog")),
            body: None,
            debug: true,
        };

        let result = analyzer.analyze(&ctx).unwrap();
        let result = result.unwrap();
        assert!(result.best_score() >= THRESHOLD);
    }
}
