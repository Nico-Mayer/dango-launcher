//! Ranking: fuzzy subsequence matching combined with a persisted frecency
//! score. Matching runs in memory on every keystroke; frecency is the only part
//! that touches storage, and only on launch, never on the search path.

mod frecency;
mod fuzzy;

use std::sync::Arc;

pub use frecency::{now_millis, FrecencyPersistence, FrecencyTable};
pub use fuzzy::{fuzzy_match, Match};

use crate::search::{Candidate, Ranker};

/// How much a candidate's frecency counts relative to raw match quality. Tuned
/// by feel; isolated here so it can change without touching the pipeline.
const FRECENCY_WEIGHT: f64 = 6.0;

/// A keyword or alias match is worth slightly less than the same match in the
/// title, so a title hit wins an otherwise equal race.
const NON_TITLE_PENALTY: i32 = 4;

pub struct DangoRanker {
    frecency: Arc<FrecencyTable>,
}

impl DangoRanker {
    pub fn new(frecency: Arc<FrecencyTable>) -> Self {
        Self { frecency }
    }
}

impl Ranker for DangoRanker {
    fn rank(&self, query: &str, candidates: Vec<Candidate>, limit: usize) -> Vec<Candidate> {
        let now = frecency::now_millis();
        if query.is_empty() {
            return rank_empty_query(&self.frecency, candidates, now, limit);
        }

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

            let Some(best) = best_field_match(query, &candidate) else {
                continue;
            };
            candidate.match_positions = best.title_positions;
            let frecency = self.frecency.score(&candidate.id, now);
            scored.push(Scored {
                candidate,
                tier: Tier::Match,
                score: best.score as f64 + FRECENCY_WEIGHT * frecency,
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
    let mut scored: Vec<(Candidate, f64)> = candidates
        .into_iter()
        .map(|c| {
            let score = frecency.score(&c.id, now);
            (c, score)
        })
        .collect();
    // Highest frecency first; on a first run every score is zero and this falls
    // back to title order, which is a sensible default list rather than empty.
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.title.cmp(&b.0.title)));
    scored.into_iter().map(|(c, _)| c).take(limit).collect()
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Tier {
    Match,
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

fn best_field_match(query: &str, candidate: &Candidate) -> Option<FieldMatch> {
    let mut best: Option<FieldMatch> = None;
    let mut consider = |score: i32, title_positions: Vec<usize>| {
        if best.as_ref().is_none_or(|b| score > b.score) {
            best = Some(FieldMatch {
                score,
                title_positions,
            });
        }
    };

    if let Some(m) = fuzzy_match(query, &candidate.title) {
        consider(m.score, m.positions);
    }
    for keyword in &candidate.keywords {
        if let Some(m) = fuzzy_match(query, keyword) {
            consider(m.score - NON_TITLE_PENALTY, Vec::new());
        }
    }
    if let Some(alias) = &candidate.alias {
        if let Some(m) = fuzzy_match(query, alias) {
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
        }
    }

    fn ranker() -> DangoRanker {
        DangoRanker::new(Arc::new(FrecencyTable::in_memory()))
    }

    #[test]
    fn frequently_used_items_rise_when_matches_are_equal() {
        let frecency = Arc::new(FrecencyTable::in_memory());
        let now = frecency::now_millis();
        for _ in 0..10 {
            frecency.record_launch("b", now);
        }
        let ranker = DangoRanker::new(frecency);

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
        let ranker = DangoRanker::new(frecency);

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
        let ranker = DangoRanker::new(frecency);
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
    }

    #[test]
    fn ranking_two_thousand_candidates_meets_the_budget() {
        let ranker = ranker();
        let candidates: Vec<Candidate> = (0..2000)
            .map(|i| candidate(&format!("id{i}"), &format!("Application Number {i}")))
            .collect();

        // Best of several runs, so a single scheduling hiccup on CI does not
        // fail a test about steady-state cost.
        let mut best = std::time::Duration::from_secs(1);
        for _ in 0..5 {
            let input = candidates.clone();
            let start = std::time::Instant::now();
            let _ = ranker.rank("appnum", input, 50);
            best = best.min(start.elapsed());
        }
        assert!(
            best < std::time::Duration::from_millis(30),
            "ranking 2000 candidates took {best:?}, over the 30ms budget"
        );
    }
}
