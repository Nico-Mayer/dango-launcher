//! The `dango.calculator` built-in extension: answers a root query that is
//! arithmetic or a unit conversion, as the first result, drawn as a card.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;

use crate::extension::{ActionOutcome, Extension, FormValues, Manifest};
use crate::protocol::Action;
use crate::search::{Candidate, RootProvider, Source};
use crate::text::{TextError, TextTarget};

pub const EXTENSION_ID: &str = "dango.calculator";

pub const ACTION_COPY: &str = "copy";
pub const ACTION_PASTE: &str = "paste";

/// fend stops a slow input here and answers nothing, so a pathological query
/// never holds up the results.
const DEADLINE: Duration = Duration::from_millis(100);

const APPROX: &str = "approx. ";

/// Root selection goes by id, and a game's id can be "730" just as an answer
/// can, so the answer's id carries a prefix no other result uses.
const ID_PREFIX: &str = "answer:";

pub(crate) fn manifest() -> Manifest {
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "Calculator".into(),
        icon: Some(format!("{}calculator", crate::extension::NAMED_ICON)),
        commands: vec![],
        root_items: true,
        services: false,
    }
}

pub struct CalculatorExtension {
    manifest: Manifest,
    text: Option<Arc<dyn TextTarget>>,
}

impl CalculatorExtension {
    pub fn new(text: Option<Arc<dyn TextTarget>>) -> Self {
        Self {
            manifest: manifest(),
            text,
        }
    }
}

impl Extension for CalculatorExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn root_provider(&self) -> Option<Arc<dyn RootProvider>> {
        Some(Arc::new(AnswerProvider))
    }

    fn perform_action(
        &self,
        item_id: &str,
        action_id: &str,
        _values: &FormValues,
    ) -> ActionOutcome {
        let item_id = item_id.strip_prefix(ID_PREFIX).unwrap_or(item_id);
        match action_id {
            ACTION_COPY => ActionOutcome::CopyToClipboard(item_id.to_string()),
            ACTION_PASTE => {
                let Some(text) = &self.text else {
                    return ActionOutcome::Failed(TextError::Unavailable.to_string());
                };
                match text.insert(item_id, None) {
                    Ok(()) => ActionOutcome::Done,
                    Err(error) => ActionOutcome::Failed(error.to_string()),
                }
            }
            _ => ActionOutcome::Failed("That action isn't available".into()),
        }
    }
}

/// What the card shows, and the plain answer that copy and paste use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub shown: String,
    pub plain: String,
}

pub fn answer(query: &str) -> Option<Answer> {
    let query = query.trim();
    if !worth_asking(query) {
        return None;
    }
    let deadline = Deadline(Instant::now() + DEADLINE);
    let result =
        fend_core::evaluate_preview_with_interrupt(query, &fend_core::Context::new(), &deadline);
    let main = result.get_main_result().trim();
    if main.is_empty() {
        return None;
    }
    Some(match main.strip_prefix(APPROX) {
        Some(plain) => Answer {
            shown: format!("≈ {plain}"),
            plain: plain.to_string(),
        },
        None => Answer {
            shown: main.to_string(),
            plain: main.to_string(),
        },
    })
}

/// fend reads more than sums: "4k" is 4000 and "1,5" is fifteen. Only a query
/// with an operator or a conversion word goes to it, and none with a comma
/// between digits, which a German keyboard would mean as a decimal point.
fn worth_asking(query: &str) -> bool {
    let chars: Vec<char> = query.chars().collect();
    let comma_in_number = chars
        .windows(3)
        .any(|window| window[0].is_ascii_digit() && window[1] == ',' && window[2].is_ascii_digit());
    if comma_in_number || looks_like_date(query) {
        return false;
    }
    let operator = query.contains(['+', '-', '*', '/', '^', '%']);
    let lower = query.to_lowercase();
    let conversion = [" to ", " in ", " as "]
        .iter()
        .any(|word| lower.contains(word));
    operator || conversion
}

/// "2024-11-04" or "4/11/2024": three numbers joined by the same "-" or "/"
/// are a date, not a sum to work out.
fn looks_like_date(query: &str) -> bool {
    ['-', '/'].iter().any(|separator| {
        let parts: Vec<&str> = query.split(*separator).collect();
        parts.len() == 3
            && parts
                .iter()
                .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
    })
}

struct Deadline(Instant);

impl fend_core::Interrupt for Deadline {
    fn should_interrupt(&self) -> bool {
        Instant::now() >= self.0
    }
}

struct AnswerProvider;

#[async_trait]
impl RootProvider for AnswerProvider {
    async fn items(&self, query: String) -> Vec<Candidate> {
        let Ok(Some(found)) = tokio::task::spawn_blocking(move || answer(&query)).await else {
            return Vec::new();
        };
        let action = |id: &str, title: &str| Action {
            id: id.into(),
            title: title.into(),
            ..Default::default()
        };
        vec![Candidate {
            extension_id: EXTENSION_ID.into(),
            id: format!("{ID_PREFIX}{}", found.plain),
            title: found.shown,
            status: None,
            icon: None,
            keywords: Vec::new(),
            alias: None,
            source: Source::Answer,
            actions: vec![
                action(ACTION_COPY, "Copy answer"),
                action(ACTION_PASTE, "Paste answer"),
            ],
            match_positions: Vec::new(),
            suggested: false,
            pinned: false,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(query: &str) -> Option<String> {
        answer(query).map(|answer| answer.shown)
    }

    #[test]
    fn sums_and_conversions_are_answered() {
        assert_eq!(shown("2+2").as_deref(), Some("4"));
        assert_eq!(shown("5 km to m").as_deref(), Some("5000 m"));
        assert_eq!(shown("1.5 * 2").as_deref(), Some("3"));
        assert_eq!(shown("17*23").as_deref(), Some("391"));
    }

    #[test]
    fn an_approximate_answer_is_marked_on_the_card_only() {
        assert_eq!(
            answer("5 km in miles"),
            Some(Answer {
                shown: "≈ 3.1068559612 miles".into(),
                plain: "3.1068559612 miles".into(),
            })
        );
    }

    #[test]
    fn names_and_commas_get_no_answer() {
        for query in [
            "firefox",
            "pi",
            "4k",
            "2fa",
            "1h",
            "1,5*2",
            "2048",
            "",
            "-5",
            "c++",
            "dango-ui",
            "9^9^9",
            "2024-11-04",
            "4/11/2024",
        ] {
            assert_eq!(shown(query), None, "{query}");
        }
    }

    #[test]
    fn copy_puts_the_answer_on_the_clipboard() {
        let calculator = CalculatorExtension::new(None);
        assert_eq!(
            calculator.perform_action("answer:4", ACTION_COPY, &FormValues::new()),
            ActionOutcome::CopyToClipboard("4".into())
        );
    }

    #[test]
    fn paste_without_a_text_exchange_says_so() {
        let calculator = CalculatorExtension::new(None);
        assert_eq!(
            calculator.perform_action("4", ACTION_PASTE, &FormValues::new()),
            ActionOutcome::Failed("Dango can't reach other apps right now".into())
        );
    }

    #[test]
    fn the_manifest_validates() {
        assert!(manifest().validate().is_ok());
    }

    #[tokio::test]
    async fn the_provider_answers_with_one_copyable_result() {
        let items = AnswerProvider.items("2+2".into()).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "answer:4");
        assert_eq!(items[0].source, Source::Answer);
        assert_eq!(items[0].actions[0].title, "Copy answer");
        assert_eq!(items[0].actions[1].title, "Paste answer");
    }
}
