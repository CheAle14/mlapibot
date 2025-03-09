use std::collections::HashSet;

use mlapibot_common::DetectedItem;

use super::{Matcher, MatcherKind};

#[derive(Debug, PartialEq)]
pub struct OrderedMatcher(pub Vec<MatcherKind>);

fn recursive_matches(
    words: &[&str],
    matchers: &[MatcherKind],
    debug: bool,
    depth: usize,
) -> Vec<DetectedItem> {
    if matchers.len() == 0 {
        return vec![];
    }

    let next = &matchers[0];

    let result = next.matches(words, debug);
    if result.len() == 0 {
        return Vec::new();
    }
    if matchers.len() == 1 {
        return result;
    }

    let mut absolute_min = usize::MAX;
    for result in &result {
        let (min, _) = result.min_max_word_indexes();
        absolute_min = absolute_min.min(min);
    }

    let next = recursive_matches(&words[absolute_min..], &matchers[1..], debug, depth + 1);

    let mut outcome = HashSet::new();
    for next in next {
        let (min, _) = next.min_max_word_indexes();
        for this in &result {
            let (this_min, _) = this.min_max_word_indexes();

            if (min + absolute_min) < this_min {
                continue;
            }

            let mut new = this.clone();
            new.score += next.score;

            for (idx, word) in &next.words {
                new.words.insert(absolute_min + idx, word.clone());
            }

            outcome.insert(new);
        }
    }

    outcome.into_iter().collect()
}

impl Matcher for OrderedMatcher {
    fn matches(&self, words: &[&str], debug: bool) -> Vec<DetectedItem> {
        let mut v = recursive_matches(words, &self.0, debug, 0);
        v.retain_mut(|d| {
            d.score /= self.0.len() as f32;
            d.score > 0.8
        });
        v.sort_unstable();

        v
    }
}

#[cfg(test)]
mod tests {
    use mlapibot_common::{DetectedItem, Words};

    use crate::matcher::{Matcher, MatcherKind, PhraseMatcher};

    use super::OrderedMatcher;

    #[test]
    pub fn test_multi_match() {
        let ordered = OrderedMatcher(vec![
            MatcherKind::Phrase(PhraseMatcher::new("hello")),
            MatcherKind::Phrase(PhraseMatcher::new("world")),
        ]);

        let text = Words::new("hello there hello some other world");
        let words = text.as_words();

        let result = ordered.matches(&words, true);

        assert_eq!(result.len(), 2);

        let mut opt_one = Some("**hello** there hello some other **world**");
        let mut opt_two = Some("hello there **hello** some other **world**");

        for result in result {
            let mut s = String::with_capacity(text.full_text().len());
            result.write_markdown(&text.as_words(), &mut s).unwrap();
            if let Some(opt) = &opt_one {
                if s == *opt {
                    opt_one = None;
                    continue;
                }
            }
            if let Some(opt) = &opt_two {
                if s == *opt {
                    opt_two = None;
                    continue;
                }
            }
            panic!("unrecognised {s:?}");
        }
        assert_eq!(opt_one, None);
        assert_eq!(opt_two, None);
    }

    #[test]
    pub fn test_sorting() {
        let ordered = OrderedMatcher(vec![
            MatcherKind::Phrase(PhraseMatcher::new("discords")),
            MatcherKind::Phrase(PhraseMatcher::new("email")),
            MatcherKind::Phrase(PhraseMatcher::new("service")),
            MatcherKind::Phrase(PhraseMatcher::new("compromised")),
        ]);

        let text = Words::new("apparently discords official email servers have been compromised and hackers are using it to sendout phishing links in officiallooking emails if you get an email from discord claiming your account has been disabled due to violating the tos but it still works when you log in do not click any of the links in the email copied from another server if you received an email from discordcom saying your account is disabled for a tos violation but the account is still functional do not click links in the email even though the email is considered valid by your email client 
the above email is a phishing attack anyall of the links in this will redirect to a session token stealer instantly compromising your discord account somehow discords email service has been compromised allowing the attacker to send authentic emails from discordcom the links in this email redirect to a separate compromised page on a subdomain on discordcom this allows javascript on the compromised page to obtain your discord session token from browser local storage and send it elsewhere this applies even if you normally use discord desktop discord web is used for server invite links to work outside of 
desktop yes this means that official discord emails cannot be trusted right now if you receive an email from discordcom always contact support instead of clicking links in the email");

        // expected: "discords email service has been compromised"
        //            127      128   129     130 131  132

        let words = text.as_words();
        let det = ordered.matches(&words, true);

        let mut expected = DetectedItem::new(1.0);
        expected.mark_match(127);
        expected.mark_match(128);
        expected.mark_match(129);
        expected.mark_match(132);

        assert_eq!(det.first(), Some(&expected));
    }

    #[test]
    pub fn test_matches() {
        let ordered = OrderedMatcher(vec![
            MatcherKind::Phrase(PhraseMatcher::new("hello")),
            MatcherKind::Phrase(PhraseMatcher::new("world")),
        ]);

        let text = Words::new("hello there some other world");
        let words = text.as_words();

        let det = &ordered.matches(&words, true)[0];

        let mut s = String::with_capacity(text.full_text().len());
        det.write_markdown(&words, &mut s).unwrap();

        assert_eq!(s, "**hello** there some other **world**");
    }

    #[test]
    pub fn test_no_match() {
        let ordered = OrderedMatcher(vec![
            MatcherKind::Phrase(PhraseMatcher::new("hello")),
            MatcherKind::Phrase(PhraseMatcher::new("world")),
            MatcherKind::Phrase(PhraseMatcher::new("another")),
        ]);

        let text = Words::new("hello there some other world");
        let words = text.as_words();

        let det = ordered.matches(&words, true);

        assert!(det.is_empty())
    }

    #[test]
    pub fn test_follows_ordering() {
        let ordered = OrderedMatcher(vec![
            MatcherKind::Phrase(PhraseMatcher::new("hello")),
            MatcherKind::Phrase(PhraseMatcher::new("world")),
        ]);

        let text = Words::new("world hello");
        let words = text.as_words();

        let det = ordered.matches(&words, true);

        assert!(det.is_empty())
    }
}
