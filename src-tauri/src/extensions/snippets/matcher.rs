//! The pure word-boundary keyword match over the monitor's recent-character
//! buffer, kept apart from the monitor and the store so it can be tested on its
//! own.

/// A snippet's id and its keyword, as the matcher weighs them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeywordEntry {
    pub id: String,
    pub keyword: String,
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Finds the snippet whose keyword the buffer ends with on a word boundary. A
/// boundary is where the keyword does not continue a word: either nothing
/// precedes it, or the character before it and the keyword's first character are
/// not both word characters, so a punctuation-led keyword like `;sig` always
/// matches. The longest matching keyword wins, so `;sig` beats a bare `sig`.
pub fn match_keyword<'a>(buffer: &str, entries: &'a [KeywordEntry]) -> Option<&'a KeywordEntry> {
    entries
        .iter()
        .filter(|entry| ends_on_boundary(buffer, &entry.keyword))
        .max_by_key(|entry| entry.keyword.chars().count())
}

fn ends_on_boundary(buffer: &str, keyword: &str) -> bool {
    if keyword.is_empty() || !buffer.ends_with(keyword) {
        return false;
    }
    let first = keyword.chars().next().unwrap();
    match buffer[..buffer.len() - keyword.len()].chars().next_back() {
        None => true,
        Some(before) => !(is_word_char(before) && is_word_char(first)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(pairs: &[(&str, &str)]) -> Vec<KeywordEntry> {
        pairs
            .iter()
            .map(|(id, keyword)| KeywordEntry {
                id: id.to_string(),
                keyword: keyword.to_string(),
            })
            .collect()
    }

    fn matched(buffer: &str, pairs: &[(&str, &str)]) -> Option<String> {
        match_keyword(buffer, &entries(pairs)).map(|entry| entry.id.clone())
    }

    #[test]
    fn matches_at_the_start_of_a_line() {
        assert_eq!(matched("sig", &[("s", "sig")]), Some("s".to_string()));
    }

    #[test]
    fn matches_after_a_space() {
        assert_eq!(matched("my sig", &[("s", "sig")]), Some("s".to_string()));
    }

    #[test]
    fn does_not_match_inside_a_word() {
        assert_eq!(matched("design", &[("s", "sig")]), None);
    }

    #[test]
    fn a_punctuation_led_keyword_matches_after_a_word() {
        assert_eq!(matched("a;sig", &[("s", ";sig")]), Some("s".to_string()));
    }

    #[test]
    fn the_longest_keyword_wins() {
        assert_eq!(
            matched("a;sig", &[("short", "sig"), ("long", ";sig")]),
            Some("long".to_string())
        );
    }

    #[test]
    fn no_match_when_the_buffer_does_not_end_with_a_keyword() {
        assert_eq!(matched("hello", &[("s", "sig")]), None);
    }
}
