//! Ranking: fuzzy subsequence matching combined with a persisted frecency
//! score and what the user chose for the typed text before. Matching runs in
//! memory on every keystroke; frecency and learned inputs are the only parts
//! that touch storage, and only on launch, never on the search path.

mod frecency;
mod fuzzy;
mod input_history;

use std::sync::Arc;

pub use frecency::{now_millis, FrecencyPersistence, FrecencyTable};
pub use fuzzy::{Match, Query};
pub use input_history::{InputHistory, InputHistoryPersistence};

use crate::search::{Candidate, Ranker};

/// How much a candidate's frecency counts relative to raw match quality. Tuned
/// by feel; isolated here so it can change without touching the pipeline.
const FRECENCY_WEIGHT: f64 = 12.0;

/// How much one past confirm for a longer text that starts with the query
/// counts. A single `cal` for Calendar should lift it over a slightly better
/// textual match on `c`, but not pin it above an item used far more often.
const LEARNED_WEIGHT: f64 = 20.0;

const SUGGESTION_LIMIT: usize = 6;

/// A keyword or alias match is worth slightly less than the same match in the
/// title, so a title hit wins an otherwise equal race.
const NON_TITLE_PENALTY: i32 = 8;

pub struct DangoRanker {
    frecency: Arc<FrecencyTable>,
    learned: Arc<InputHistory>,
}

impl DangoRanker {
    pub fn new(frecency: Arc<FrecencyTable>, learned: Arc<InputHistory>) -> Self {
        Self { frecency, learned }
    }
}

impl Ranker for DangoRanker {
    fn rank(&self, query: &str, candidates: Vec<Candidate>, limit: usize) -> Vec<Candidate> {
        let now = frecency::now_millis();
        if query.is_empty() {
            return rank_empty_query(&self.frecency, candidates, now, limit);
        }

        let learned = self.learned.lookup(query, now);
        let mut matcher = Query::new(query);
        let mut scored: Vec<Scored> = Vec::new();
        for mut candidate in candidates {
            // An exact alias match is a deliberate, unambiguous request, so it
            // outranks everything regardless of match score or frecency.
            if candidate
                .alias
                .as_deref()
                .is_some_and(|alias| alias.eq_ignore_ascii_case(query))
            {
                candidate.match_positions.clear();
                scored.push(Scored {
                    candidate,
                    tier: Tier::AliasExact,
                    score: f64::INFINITY,
                });
                continue;
            }

            // A text the user already resolved to this item needs no textual
            // argument: it ranks on how established that choice is.
            if let Some(weight) = learned.exact.get(&candidate.id) {
                candidate.match_positions = best_field_match(&mut matcher, &candidate)
                    .map(|best| best.title_positions)
                    .unwrap_or_default();
                scored.push(Scored {
                    candidate,
                    tier: Tier::LearnedExact,
                    score: *weight,
                });
                continue;
            }

            let Some(best) = best_field_match(&mut matcher, &candidate) else {
                continue;
            };
            candidate.match_positions = best.title_positions;
            let frecency = self.frecency.score(&candidate.id, now);
            let prefix = learned.prefix.get(&candidate.id).copied().unwrap_or(0.0);
            scored.push(Scored {
                candidate,
                tier: Tier::Match,
                score: best.score as f64 + FRECENCY_WEIGHT * frecency + LEARNED_WEIGHT * prefix,
            });
        }

        scored.sort_by(|a, b| {
            b.tier
                .cmp(&a.tier)
                .then_with(|| b.score.total_cmp(&a.score))
                .then_with(|| a.candidate.title.cmp(&b.candidate.title))
        });
        scored
            .into_iter()
            .map(|s| s.candidate)
            .take(limit)
            .collect()
    }
}

fn rank_empty_query(
    frecency: &FrecencyTable,
    candidates: Vec<Candidate>,
    now: i64,
    limit: usize,
) -> Vec<Candidate> {
    let (pinned, rest): (Vec<Candidate>, Vec<Candidate>) =
        candidates.into_iter().partition(|c| c.pinned);
    let mut scored: Vec<(Candidate, f64)> = rest
        .into_iter()
        .map(|c| {
            let score = frecency.score(&c.id, now);
            (c, score)
        })
        .collect();
    // Highest frecency first; on a first run every score is zero and this falls
    // back to title order, which is a sensible default list rather than empty.
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.title.cmp(&b.0.title)));
    let ranked = scored
        .into_iter()
        .enumerate()
        .map(|(index, (mut c, score))| {
            c.suggested = index < SUGGESTION_LIMIT && score > 0.0;
            c
        });
    pinned.into_iter().chain(ranked).take(limit).collect()
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Tier {
    Match,
    LearnedExact,
    AliasExact,
}

struct Scored {
    candidate: Candidate,
    tier: Tier,
    score: f64,
}

struct FieldMatch {
    score: i32,
    /// Positions are kept only for a title match, since that is what the list
    /// renders and highlights.
    title_positions: Vec<usize>,
}

fn best_field_match(query: &mut Query, candidate: &Candidate) -> Option<FieldMatch> {
    let mut best: Option<FieldMatch> = None;
    let mut consider = |score: i32, title_positions: Vec<usize>| {
        if best.as_ref().is_none_or(|b| score > b.score) {
            best = Some(FieldMatch {
                score,
                title_positions,
            });
        }
    };

    if let Some(m) = query.score(&candidate.title) {
        consider(m.score, m.positions);
    }
    for keyword in &candidate.keywords {
        if let Some(m) = query.score(keyword) {
            consider(m.score - NON_TITLE_PENALTY, Vec::new());
        }
    }
    if let Some(alias) = &candidate.alias {
        if let Some(m) = query.score(alias) {
            consider(m.score - NON_TITLE_PENALTY, Vec::new());
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::Source;

    fn candidate(id: &str, title: &str) -> Candidate {
        Candidate {
            extension_id: "test".into(),
            id: id.into(),
            title: title.into(),
            subtitle: None,
            icon: None,
            keywords: vec![],
            alias: None,
            source: Source::Command,
            actions: vec![],
            match_positions: vec![],
            suggested: false,
            pinned: false,
        }
    }

    fn ranker() -> DangoRanker {
        DangoRanker::new(
            Arc::new(FrecencyTable::in_memory()),
            Arc::new(InputHistory::in_memory()),
        )
    }

    fn ranker_with(frecency: Arc<FrecencyTable>, learned: Arc<InputHistory>) -> DangoRanker {
        DangoRanker::new(frecency, learned)
    }

    fn learned_ranker(learned: Arc<InputHistory>) -> DangoRanker {
        DangoRanker::new(Arc::new(FrecencyTable::in_memory()), learned)
    }

    #[test]
    fn frequently_used_items_rise_when_matches_are_equal() {
        let frecency = Arc::new(FrecencyTable::in_memory());
        let now = frecency::now_millis();
        for _ in 0..10 {
            frecency.record_launch("b", now);
        }
        let ranker = ranker_with(frecency, Arc::new(InputHistory::in_memory()));

        let ranked = ranker.rank(
            "app",
            vec![candidate("a", "Apple"), candidate("b", "Apple")],
            10,
        );
        assert_eq!(ranked[0].id, "b");
    }

    #[test]
    fn recent_use_beats_stale_use_at_equal_count() {
        let frecency = Arc::new(FrecencyTable::in_memory());
        let now = frecency::now_millis();
        let month = 30 * 24 * 60 * 60 * 1000;
        frecency.record_launch("stale", now - 6 * month);
        frecency.record_launch("fresh", now);
        let ranker = ranker_with(frecency, Arc::new(InputHistory::in_memory()));

        let ranked = ranker.rank(
            "app",
            vec![candidate("stale", "Apple"), candidate("fresh", "Apple")],
            10,
        );
        assert_eq!(ranked[0].id, "fresh");
    }

    #[test]
    fn an_exact_alias_is_ranked_first() {
        let mut gmail = candidate("mail", "Some Unrelated Thing");
        gmail.alias = Some("gm".into());
        let ranker = ranker();
        let ranked = ranker.rank("gm", vec![candidate("game", "Game"), gmail], 10);
        assert_eq!(ranked[0].id, "mail");
    }

    #[test]
    fn a_subsequence_matches_and_reports_positions() {
        let ranker = ranker();
        let ranked = ranker.rank("cde", vec![candidate("x", "VS Code Editor")], 10);
        assert_eq!(ranked.len(), 1);
        assert!(
            !ranked[0].match_positions.is_empty(),
            "matched positions should be reported for highlighting"
        );
    }

    #[test]
    fn a_word_start_beats_a_mid_word_hit_at_equal_frecency() {
        let ranker = ranker();
        let ranked = ranker.rank(
            "ze",
            vec![candidate("max", "Maximize"), candidate("zed", "Zed")],
            10,
        );
        assert_eq!(ranked[0].id, "zed");
    }

    #[test]
    fn a_contiguous_match_beats_a_scattered_one_at_equal_frecency() {
        let ranker = ranker();
        let ranked = ranker.rank(
            "fire",
            vec![
                candidate("far", "Far Infrared Emitter"),
                candidate("fox", "Firefox"),
            ],
            10,
        );
        assert_eq!(ranked[0].id, "fox");
    }

    #[test]
    fn a_keyword_widens_matching_beyond_the_title() {
        let mut c = candidate("term", "Terminal");
        c.keywords = vec!["shell".into()];
        let ranker = ranker();
        let ranked = ranker.rank("shell", vec![c], 10);
        assert_eq!(ranked.len(), 1);
    }

    #[test]
    fn non_matching_candidates_are_dropped() {
        let ranker = ranker();
        let ranked = ranker.rank("zzz", vec![candidate("a", "Apple")], 10);
        assert!(ranked.is_empty());
    }

    #[test]
    fn empty_query_orders_by_frecency() {
        let frecency = Arc::new(FrecencyTable::in_memory());
        let now = frecency::now_millis();
        frecency.record_launch("used", now);
        let ranker = ranker_with(frecency, Arc::new(InputHistory::in_memory()));
        let ranked = ranker.rank(
            "",
            vec![candidate("cold", "Cold"), candidate("used", "Used")],
            10,
        );
        assert_eq!(ranked[0].id, "used");
    }

    #[test]
    fn empty_query_on_first_run_falls_back_to_title_order() {
        let ranker = ranker();
        let ranked = ranker.rank(
            "",
            vec![candidate("b", "Beta"), candidate("a", "Alpha")],
            10,
        );
        assert_eq!(ranked[0].id, "a");
        assert!(ranked.iter().all(|c| !c.suggested));
    }

    #[test]
    fn empty_query_marks_launched_items_as_suggestions() {
        let frecency = Arc::new(FrecencyTable::in_memory());
        frecency.record_launch("used", frecency::now_millis());
        let ranker = ranker_with(frecency, Arc::new(InputHistory::in_memory()));
        let ranked = ranker.rank(
            "",
            vec![candidate("cold", "Cold"), candidate("used", "Used")],
            10,
        );
        assert!(ranked[0].suggested);
        assert!(!ranked[1].suggested);
    }

    #[test]
    fn suggestions_are_capped_and_the_rest_keep_frecency_order() {
        let frecency = Arc::new(FrecencyTable::in_memory());
        let now = frecency::now_millis();
        let mut candidates: Vec<Candidate> = (0..8)
            .map(|i| candidate(&format!("used{i}"), &format!("Used {i}")))
            .collect();
        for (i, c) in candidates.iter().enumerate() {
            for _ in 0..(8 - i) {
                frecency.record_launch(&c.id, now);
            }
        }
        candidates.push(candidate("aaa", "Aaa Never Launched"));
        let ranker = ranker_with(frecency, Arc::new(InputHistory::in_memory()));

        let ranked = ranker.rank("", candidates, 10);
        let suggested: Vec<&str> = ranked
            .iter()
            .filter(|c| c.suggested)
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(
            suggested,
            ["used0", "used1", "used2", "used3", "used4", "used5"]
        );
        assert_eq!(ranked[6].id, "used6");
        assert_eq!(ranked[7].id, "used7");
        assert!(!ranked[6].suggested && !ranked[7].suggested);
        assert_eq!(ranked[8].id, "aaa");
    }

    #[test]
    fn pinned_items_lead_an_empty_query_in_provider_order() {
        let frecency = Arc::new(FrecencyTable::in_memory());
        frecency.record_launch("used", frecency::now_millis());
        let ranker = ranker_with(frecency, Arc::new(InputHistory::in_memory()));
        let mut second = candidate("pin-b", "Zeta");
        second.pinned = true;
        let mut first = candidate("pin-a", "Omega");
        first.pinned = true;

        let ranked = ranker.rank("", vec![candidate("used", "Used"), first, second], 10);

        let ids: Vec<&str> = ranked.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["pin-a", "pin-b", "used"]);
        assert!(!ranked[0].suggested && !ranked[1].suggested);
        assert!(ranked[2].suggested);
    }

    #[test]
    fn pinned_has_no_effect_on_a_typed_query() {
        let ranker = ranker();
        let mut pinned = candidate("pin", "Accessibility is off");
        pinned.pinned = true;

        let ranked = ranker.rank("alpha", vec![pinned, candidate("a", "Alpha")], 10);

        let ids: Vec<&str> = ranked.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["a"]);
    }

    #[test]
    fn a_query_marks_nothing_as_suggested() {
        let frecency = Arc::new(FrecencyTable::in_memory());
        frecency.record_launch("used", frecency::now_millis());
        let ranker = ranker_with(frecency, Arc::new(InputHistory::in_memory()));
        let ranked = ranker.rank("use", vec![candidate("used", "Used")], 10);
        assert!(!ranked[0].suggested);
    }

    #[test]
    fn a_learned_exact_input_wins_over_text_and_frecency() {
        let frecency = Arc::new(FrecencyTable::in_memory());
        let learned = Arc::new(InputHistory::in_memory());
        let now = frecency::now_millis();
        for _ in 0..20 {
            frecency.record_launch("calc", now);
        }
        learned.record("cal", "calendar", now);
        let ranker = ranker_with(frecency, learned);

        let ranked = ranker.rank(
            "cal",
            vec![
                candidate("calc", "Calculator"),
                candidate("calendar", "Kalender"),
            ],
            10,
        );
        assert_eq!(ranked[0].id, "calendar");
    }

    #[test]
    fn a_learned_prefix_boosts_over_a_slightly_better_match() {
        let learned = Arc::new(InputHistory::in_memory());
        learned.record("cal", "calendar", frecency::now_millis());
        let ranker = learned_ranker(learned);

        let ranked = ranker.rank(
            "c",
            vec![
                candidate("chrome", "Chrome"),
                candidate("calendar", "Calendar"),
            ],
            10,
        );
        assert_eq!(ranked[0].id, "calendar");
        assert_eq!(ranked[1].id, "chrome");
    }

    #[test]
    fn a_learned_prefix_does_not_pin_over_heavy_use() {
        let frecency = Arc::new(FrecencyTable::in_memory());
        let learned = Arc::new(InputHistory::in_memory());
        let now = frecency::now_millis();
        for _ in 0..10 {
            frecency.record_launch("chrome", now);
        }
        learned.record("cal", "calendar", now);
        let ranker = ranker_with(frecency, learned);

        let ranked = ranker.rank(
            "c",
            vec![
                candidate("calendar", "Calendar"),
                candidate("chrome", "Chrome"),
            ],
            10,
        );
        assert_eq!(ranked[0].id, "chrome");
    }

    #[test]
    fn typing_past_a_learned_input_releases_it() {
        let learned = Arc::new(InputHistory::in_memory());
        learned.record("cal", "calendar", frecency::now_millis());
        let ranker = learned_ranker(learned);

        let ranked = ranker.rank(
            "calc",
            vec![
                candidate("calendar", "Calendar"),
                candidate("calc", "Calculator"),
            ],
            10,
        );
        assert_eq!(ranked[0].id, "calc");
    }

    #[test]
    fn a_repeated_correction_wins() {
        let learned = Arc::new(InputHistory::in_memory());
        let now = frecency::now_millis();
        learned.record("cal", "calendar", now);
        for _ in 0..3 {
            learned.record("cal", "calc", now);
        }
        let ranker = learned_ranker(learned);

        let ranked = ranker.rank(
            "cal",
            vec![
                candidate("calendar", "Calendar"),
                candidate("calc", "Calculator"),
            ],
            10,
        );
        assert_eq!(ranked[0].id, "calc");
    }

    #[test]
    fn a_single_slip_does_not_win() {
        let learned = Arc::new(InputHistory::in_memory());
        let now = frecency::now_millis();
        for _ in 0..10 {
            learned.record("cal", "calendar", now);
        }
        learned.record("cal", "calc", now);
        let ranker = learned_ranker(learned);

        let ranked = ranker.rank(
            "cal",
            vec![
                candidate("calc", "Calculator"),
                candidate("calendar", "Calendar"),
            ],
            10,
        );
        assert_eq!(ranked[0].id, "calendar");
    }

    #[test]
    fn an_unused_pair_fades_behind_a_recent_one() {
        let learned = Arc::new(InputHistory::in_memory());
        let now = frecency::now_millis();
        let month = 30 * 24 * 60 * 60 * 1000;
        for _ in 0..3 {
            learned.record("cal", "calendar", now - 6 * month);
        }
        learned.record("cal", "calc", now);
        let ranker = learned_ranker(learned);

        let ranked = ranker.rank(
            "cal",
            vec![
                candidate("calendar", "Calendar"),
                candidate("calc", "Calculator"),
            ],
            10,
        );
        assert_eq!(ranked[0].id, "calc");
    }

    #[test]
    fn an_alias_still_beats_a_learned_input() {
        let learned = Arc::new(InputHistory::in_memory());
        learned.record("gm", "game", frecency::now_millis());
        let ranker = learned_ranker(learned);
        let mut mail = candidate("mail", "Mail");
        mail.alias = Some("gm".into());

        let ranked = ranker.rank("gm", vec![candidate("game", "Game"), mail], 10);
        assert_eq!(ranked[0].id, "mail");
        assert_eq!(ranked[1].id, "game");
    }

    #[test]
    fn learned_inputs_leave_the_empty_query_alone() {
        let frecency = Arc::new(FrecencyTable::in_memory());
        let learned = Arc::new(InputHistory::in_memory());
        let now = frecency::now_millis();
        frecency.record_launch("used", now);
        for _ in 0..5 {
            learned.record("c", "cold", now);
        }
        let ranker = ranker_with(frecency, learned);

        let ranked = ranker.rank(
            "",
            vec![candidate("cold", "Cold"), candidate("used", "Used")],
            10,
        );
        assert_eq!(ranked[0].id, "used");
        assert!(!ranked[1].suggested);
    }
}
