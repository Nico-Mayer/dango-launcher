//! Fuzzy matching through nucleo-matcher, the engine behind Helix: fzf
//! scoring with word-boundary bonuses, case folding and accent normalisation,
//! and the matched character positions so the frontend can highlight them.

use nucleo_matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization};
use nucleo_matcher::{Config, Matcher, Utf32Str};

pub struct Match {
    pub score: i32,
    pub positions: Vec<usize>,
}

/// One typed query, ready to score any number of texts. Owns the matcher's
/// scratch buffers, so it is built once per keystroke, not per candidate.
pub struct Query {
    atom: Atom,
    matcher: Matcher,
    haystack: Vec<char>,
    indices: Vec<u32>,
}

impl Query {
    pub fn new(query: &str) -> Self {
        let mut config = Config::DEFAULT;
        // The crate documents this for input where the user types the start
        // of the match, which is what a launcher query is.
        config.prefer_prefix = true;
        Self {
            atom: Atom::new(
                query,
                CaseMatching::Ignore,
                Normalization::Smart,
                AtomKind::Fuzzy,
                false,
            ),
            matcher: Matcher::new(config),
            haystack: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// `Some` when every character of the query appears in `text` in order.
    /// An empty query matches everything with a neutral score.
    pub fn score(&mut self, text: &str) -> Option<Match> {
        let haystack = Utf32Str::new(text, &mut self.haystack);
        self.indices.clear();
        let score = self
            .atom
            .indices(haystack, &mut self.matcher, &mut self.indices)?;
        self.indices.sort_unstable();
        self.indices.dedup();
        Some(Match {
            score: i32::from(score),
            positions: self.indices.iter().map(|&i| i as usize).collect(),
        })
    }
}

#[cfg(test)]
pub fn fuzzy_match(query: &str, text: &str) -> Option<Match> {
    Query::new(query).score(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_case_insensitive() {
        assert!(fuzzy_match("CODE", "Visual Studio Code").is_some());
    }

    #[test]
    fn positions_point_at_matched_characters() {
        let m = fuzzy_match("vsc", "Visual Studio Code").unwrap();
        assert_eq!(m.positions, vec![0, 7, 14]);
    }

    #[test]
    fn an_accented_title_matches_an_unaccented_query_by_character() {
        let m = fuzzy_match("cafe", "Café").unwrap();
        assert_eq!(m.positions, vec![0, 1, 2, 3]);
    }

    #[test]
    fn pattern_syntax_is_taken_literally() {
        assert!(fuzzy_match("^fire", "Firefox").is_none());
        assert!(fuzzy_match("^fire", "^Firefox").is_some());
    }

    #[test]
    fn an_empty_query_matches_everything() {
        let m = fuzzy_match("", "Anything").unwrap();
        assert_eq!(m.score, 0);
        assert!(m.positions.is_empty());
    }

    #[test]
    fn one_query_scores_many_texts() {
        let mut query = Query::new("fi");
        assert!(query.score("Firefox").is_some());
        assert!(query.score("Nothing").is_none());
        assert!(query.score("Finder").is_some());
    }
}
