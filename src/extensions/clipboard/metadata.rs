//! What the detail pane says about an entry beyond its content.

use std::path::Path;

use time::macros::format_description;
use time::{Duration, OffsetDateTime};

use super::history::LISTED_CHARS;

/// The "Type" of a text entry. Only a single token without spaces is ever
/// recognised, so a sentence containing a link is still text.
pub fn text_kind(text: &str) -> &'static str {
    let text = text.trim();
    if text.is_empty() || text.chars().count() >= LISTED_CHARS || text.contains(char::is_whitespace)
    {
        return "Text";
    }
    if is_link(text) {
        "Link"
    } else if is_email(text) {
        "Email address"
    } else if is_hex_colour(text) {
        "Colour"
    } else if Path::new(text).is_absolute() {
        "File path"
    } else {
        "Text"
    }
}

fn is_link(text: &str) -> bool {
    url::Url::parse(text).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

fn is_email(text: &str) -> bool {
    !text.starts_with("mailto:") && email_address::EmailAddress::is_valid(text)
}

fn is_hex_colour(text: &str) -> bool {
    text.strip_prefix('#').is_some_and(|digits| {
        matches!(digits.len(), 3 | 4 | 6 | 8) && digits.chars().all(|c| c.is_ascii_hexdigit())
    })
}

/// "Copied" for an entry copied at `at` (milliseconds since the epoch), as
/// seen at `now` in its offset.
pub fn copied(at: i64, now: OffsetDateTime) -> String {
    let at = (OffsetDateTime::UNIX_EPOCH + Duration::milliseconds(at)).to_offset(now.offset());
    let time = format_description!("[hour]:[minute]");
    let (day, format) = if at.date() == now.date() {
        ("Today, ", time)
    } else if Some(at.date()) == now.date().previous_day() {
        ("Yesterday, ", time)
    } else if at.year() == now.year() {
        (
            "",
            format_description!("[day padding:none] [month repr:short], [hour]:[minute]"),
        )
    } else {
        (
            "",
            format_description!("[day padding:none] [month repr:short] [year], [hour]:[minute]"),
        )
    };
    format!("{day}{}", at.format(format).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn a_web_address_is_a_link() {
        assert_eq!(text_kind("https://example.com/a?b=c"), "Link");
        assert_eq!(text_kind("  http://example.com\n"), "Link");
    }

    #[test]
    fn another_scheme_is_not_a_link() {
        assert_eq!(text_kind("ftp://example.com"), "Text");
    }

    #[test]
    fn a_sentence_containing_a_link_is_text() {
        assert_eq!(text_kind("see https://example.com for more"), "Text");
    }

    #[test]
    fn an_address_is_an_email_address() {
        assert_eq!(text_kind("someone@example.com"), "Email address");
        assert_eq!(text_kind("mailto:someone@example.com"), "Text");
    }

    #[test]
    fn hex_colours_are_colours() {
        for colour in ["#f80", "#f80a", "#ff8800", "#FF8800cc"] {
            assert_eq!(text_kind(colour), "Colour", "{colour}");
        }
        for not in ["#ff888", "#gg8800", "#", "ff8800"] {
            assert_eq!(text_kind(not), "Text", "{not}");
        }
    }

    #[test]
    fn a_colour_name_is_text() {
        assert_eq!(text_kind("red"), "Text");
    }

    #[test]
    fn an_absolute_path_is_a_file_path() {
        let path = std::env::temp_dir().join("notes.txt");
        assert_eq!(text_kind(path.to_str().unwrap()), "File path");
        assert_eq!(text_kind("notes.txt"), "Text");
    }

    #[test]
    fn a_shell_namespace_identifier_is_text() {
        assert_eq!(
            text_kind("::{20D04FE0-3AEA-1069-A2D8-08002B30309D}"),
            "Text"
        );
    }

    #[test]
    fn several_lines_are_text() {
        assert_eq!(text_kind("https://a.com\nhttps://b.com"), "Text");
    }

    const NOW: OffsetDateTime = datetime!(2026-11-01 16:00 +01:00);

    fn millis(at: OffsetDateTime) -> i64 {
        (at.unix_timestamp_nanos() / 1_000_000) as i64
    }

    #[test]
    fn today_shows_the_local_time() {
        let at = millis(datetime!(2026-11-01 13:31 UTC));
        assert_eq!(copied(at, NOW), "Today, 14:31");
    }

    #[test]
    fn yesterday_says_so() {
        let at = millis(datetime!(2026-10-31 14:31 +01:00));
        assert_eq!(copied(at, NOW), "Yesterday, 14:31");
    }

    #[test]
    fn earlier_this_year_shows_day_and_month() {
        let at = millis(datetime!(2026-10-27 09:05 +01:00));
        assert_eq!(copied(at, NOW), "27 Oct, 09:05");
    }

    #[test]
    fn an_earlier_year_shows_the_year() {
        let at = millis(datetime!(2025-10-27 14:31 +01:00));
        assert_eq!(copied(at, NOW), "27 Oct 2025, 14:31");
    }
}
