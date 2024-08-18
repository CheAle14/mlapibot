use ord_many::max_many;
use serde::Deserialize;

use crate::{analysis::DetectedItem, utils::Words};

use super::Matcher;

#[derive(Debug, PartialEq)]
pub struct PhraseMatcher(Words);

impl PhraseMatcher {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let words = Words::new(text);
        Self(words)
    }
}

impl Matcher for PhraseMatcher {
    fn matches(&self, haystack: &[&str], debug: bool) -> Option<DetectedItem> {
        let words = self.0.as_words();

        if words.len() == 1 {
            if debug {
                println!("  Looking for single {:?}", words[0])
            }

            let mut best_score = 0.0;
            let mut best_idx = 0;

            for (idx, word) in haystack.iter().enumerate() {
                if words[0] == *word {
                    let mut d = DetectedItem::new(1.0);
                    d.mark_match(idx);
                    return Some(d);
                }

                let sim = string_similiarity(words[0], word);
                if sim > best_score {
                    best_score = sim;
                    best_idx = idx;
                }
            }

            if best_score > THRESHOLD {
                let mut d = DetectedItem::new(best_score);
                d.mark_match(best_idx);
                return Some(d);
            } else {
                return None;
            }
        }

        if debug {
            println!("  Looking for {:?}", self.0.full_text())
        }

        let alignment = needleman_wunsch(&words, haystack);
        let score = score(&alignment, &words, haystack, debug);

        if debug {
            println!("  Score: {score}");
        }

        if score < THRESHOLD {
            return None;
        }

        let mut item = DetectedItem::new(score);
        for entry in alignment.iter() {
            match entry {
                AlignmentKind::Matches { j, .. } => item.mark_match(*j),
                AlignmentKind::MissingI { .. } => (),
                AlignmentKind::MissingJ { .. } => (),
            }
        }

        Some(item)
    }
}

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        analysis::{str_analyzer::StrAnalzyer, str_matchers::MatcherKind},
        context::{Context, ContextKind},
    };

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
            title: Some(MatcherKind::Phrase(PhraseMatcher(Words::new(phrase)))),
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
