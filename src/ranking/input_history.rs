//! Learned inputs: which item the user chose after typing a given text. A
//! stored input fires when the typed text is a prefix of it, so the longer
//! text that disambiguated once teaches the shorter one. Held in memory like
//! frecency; ranking never reads the database.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use super::frecency::HALF_LIFE_MS;

#[derive(Clone, Copy)]
pub struct Record {
    pub use_count: u32,
    pub last_used_at: i64,
}

pub trait InputHistoryPersistence: Send + Sync {
    fn load_all(&self) -> Vec<(String, String, String, Record)>;
    fn save(&self, input: &str, extension_id: &str, item_id: &str, record: Record);
}

/// Weights keyed by extension id, then item id.
type ByItem = HashMap<String, HashMap<String, f64>>;

/// What the learned pairs say about one typed text, per item.
#[derive(Default)]
pub struct Learned {
    /// The text was typed exactly before and this item chosen for it.
    exact: ByItem,
    /// Summed weight of every stored input that starts with the text.
    prefix: ByItem,
}

impl Learned {
    pub fn exact(&self, extension_id: &str, item_id: &str) -> Option<f64> {
        get(&self.exact, extension_id, item_id)
    }

    pub fn prefix(&self, extension_id: &str, item_id: &str) -> Option<f64> {
        get(&self.prefix, extension_id, item_id)
    }
}

fn get(weights: &ByItem, extension_id: &str, item_id: &str) -> Option<f64> {
    weights.get(extension_id)?.get(item_id).copied()
}

struct Pair {
    extension_id: String,
    item_id: String,
    record: Record,
}

pub struct InputHistory {
    pairs: Mutex<BTreeMap<String, Vec<Pair>>>,
    persistence: Option<Box<dyn InputHistoryPersistence>>,
}

impl InputHistory {
    pub fn load(persistence: Box<dyn InputHistoryPersistence>) -> Self {
        let mut pairs: BTreeMap<String, Vec<Pair>> = BTreeMap::new();
        for (input, extension_id, item_id, record) in persistence.load_all() {
            pairs.entry(input).or_default().push(Pair {
                extension_id,
                item_id,
                record,
            });
        }
        Self {
            pairs: Mutex::new(pairs),
            persistence: Some(persistence),
        }
    }

    pub fn in_memory() -> Self {
        Self {
            pairs: Mutex::new(BTreeMap::new()),
            persistence: None,
        }
    }

    pub fn record(&self, input: &str, extension_id: &str, item_id: &str, now: i64) {
        if let Some((input, record)) = self.note(input, extension_id, item_id, now) {
            self.persist(&input, extension_id, item_id, record);
        }
    }

    pub fn note(
        &self,
        input: &str,
        extension_id: &str,
        item_id: &str,
        now: i64,
    ) -> Option<(String, Record)> {
        let input = normalise(input);
        if input.is_empty() {
            return None;
        }
        let record = {
            let mut pairs = self.pairs.lock().unwrap();
            let items = pairs.entry(input.clone()).or_default();
            let position = match items
                .iter()
                .position(|pair| pair.extension_id == extension_id && pair.item_id == item_id)
            {
                Some(position) => position,
                None => {
                    items.push(Pair {
                        extension_id: extension_id.to_string(),
                        item_id: item_id.to_string(),
                        record: Record {
                            use_count: 0,
                            last_used_at: now,
                        },
                    });
                    items.len() - 1
                }
            };
            let record = &mut items[position].record;
            record.use_count += 1;
            record.last_used_at = now;
            *record
        };
        Some((input, record))
    }

    pub fn persist(&self, input: &str, extension_id: &str, item_id: &str, record: Record) {
        if let Some(persistence) = &self.persistence {
            persistence.save(input, extension_id, item_id, record);
        }
    }

    pub fn lookup(&self, query: &str, now: i64) -> Learned {
        let query = normalise(query);
        let mut learned = Learned::default();
        if query.is_empty() {
            return learned;
        }
        let pairs = self.pairs.lock().unwrap();
        for (input, items) in pairs.range(query.clone()..) {
            if !input.starts_with(&query) {
                break;
            }
            for pair in items {
                let weight = decayed(&pair.record, now);
                *learned
                    .prefix
                    .entry(pair.extension_id.clone())
                    .or_default()
                    .entry(pair.item_id.clone())
                    .or_default() += weight;
                if *input == query {
                    learned
                        .exact
                        .entry(pair.extension_id.clone())
                        .or_default()
                        .insert(pair.item_id.clone(), weight);
                }
            }
        }
        learned
    }
}

fn normalise(input: &str) -> String {
    input.trim().to_lowercase()
}

fn decayed(record: &Record, now: i64) -> f64 {
    let age = (now - record.last_used_at).max(0) as f64;
    record.use_count as f64 * 0.5_f64.powf(age / HALF_LIFE_MS)
}

impl InputHistoryPersistence for std::sync::Arc<crate::store::Store> {
    fn load_all(&self) -> Vec<(String, String, String, Record)> {
        self.load_input_history()
            .unwrap_or_default()
            .into_iter()
            .map(|(input, extension_id, item_id, use_count, last_used_at)| {
                (
                    input,
                    extension_id,
                    item_id,
                    Record {
                        use_count,
                        last_used_at,
                    },
                )
            })
            .collect()
    }

    fn save(&self, input: &str, extension_id: &str, item_id: &str, record: Record) {
        if let Err(error) = self.save_input_history(
            input,
            extension_id,
            item_id,
            record.use_count,
            record.last_used_at,
        ) {
            eprintln!("[dango] could not persist the input history for {item_id}: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ranking::now_millis;

    #[test]
    fn case_and_whitespace_fold_into_one_input() {
        let history = InputHistory::in_memory();
        let now = now_millis();
        history.record("Cal", "ext", "calendar", now);
        history.record(" cal", "ext", "calendar", now);
        let learned = history.lookup("cal", now);
        assert_eq!(learned.exact("ext", "calendar").unwrap(), 2.0);
        assert!(history
            .lookup("CAL ", now)
            .exact("ext", "calendar")
            .is_some());
    }

    #[test]
    fn an_empty_input_records_nothing_and_matches_nothing() {
        let history = InputHistory::in_memory();
        let now = now_millis();
        history.record("   ", "ext", "calendar", now);
        history.record("cal", "ext", "calendar", now);
        assert!(history.lookup("", now).prefix.is_empty());
        assert_eq!(history.pairs.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_shorter_query_sees_the_pair_as_a_prefix_not_an_exact() {
        let history = InputHistory::in_memory();
        let now = now_millis();
        history.record("cal", "ext", "calendar", now);

        let exact = history.lookup("cal", now);
        assert_eq!(exact.exact("ext", "calendar").unwrap(), 1.0);
        assert_eq!(exact.prefix("ext", "calendar").unwrap(), 1.0);

        let prefix = history.lookup("c", now);
        assert!(prefix.exact.is_empty());
        assert_eq!(prefix.prefix("ext", "calendar").unwrap(), 1.0);
    }

    #[test]
    fn typing_past_the_stored_input_releases_it() {
        let history = InputHistory::in_memory();
        let now = now_millis();
        history.record("cal", "ext", "calendar", now);
        let learned = history.lookup("calc", now);
        assert!(learned.exact.is_empty());
        assert!(learned.prefix.is_empty());
    }

    #[test]
    fn only_inputs_starting_with_the_query_count() {
        let history = InputHistory::in_memory();
        let now = now_millis();
        history.record("cal", "ext", "calendar", now);
        history.record("chrome", "ext", "chrome", now);
        history.record("dal", "ext", "other", now);
        let learned = history.lookup("c", now);
        assert_eq!(learned.prefix["ext"].len(), 2);
        assert!(learned.prefix("ext", "other").is_none());
    }

    #[test]
    fn the_same_item_id_in_two_extensions_learns_apart() {
        let history = InputHistory::in_memory();
        let now = now_millis();
        history.record("cal", "one", "item", now);
        let learned = history.lookup("cal", now);
        assert!(learned.exact("one", "item").is_some());
        assert!(learned.exact("two", "item").is_none());
        assert!(learned.prefix("two", "item").is_none());
    }

    #[test]
    fn several_longer_inputs_add_up_for_one_item() {
        let history = InputHistory::in_memory();
        let now = now_millis();
        history.record("cal", "ext", "calendar", now);
        history.record("calen", "ext", "calendar", now);
        assert_eq!(
            history.lookup("c", now).prefix("ext", "calendar").unwrap(),
            2.0
        );
    }

    #[test]
    fn an_old_pair_decays() {
        let history = InputHistory::in_memory();
        let now = now_millis();
        let month = 30 * 24 * 60 * 60 * 1000;
        for _ in 0..3 {
            history.record("cal", "ext", "stale", now - 6 * month);
        }
        history.record("cal", "ext", "fresh", now);
        let learned = history.lookup("cal", now);
        assert!(learned.exact("ext", "fresh").unwrap() > learned.exact("ext", "stale").unwrap());
    }

    #[test]
    fn pairs_are_persisted_and_loaded_back() {
        use std::sync::Arc;
        const NOW: i64 = 1_700_000_000_000;
        #[derive(Default)]
        struct Spy(Mutex<Vec<(String, String, String, u32)>>);
        impl InputHistoryPersistence for Arc<Spy> {
            fn load_all(&self) -> Vec<(String, String, String, Record)> {
                vec![(
                    "seed".into(),
                    "ext".into(),
                    "item".into(),
                    Record {
                        use_count: 2,
                        last_used_at: NOW,
                    },
                )]
            }
            fn save(&self, input: &str, extension_id: &str, item_id: &str, record: Record) {
                self.0.lock().unwrap().push((
                    input.into(),
                    extension_id.into(),
                    item_id.into(),
                    record.use_count,
                ));
            }
        }
        let spy = Arc::new(Spy::default());
        let history = InputHistory::load(Box::new(spy.clone()));
        assert_eq!(
            history.lookup("seed", NOW).exact("ext", "item").unwrap(),
            2.0
        );
        history.record("Seed", "ext", "item", NOW);
        assert_eq!(
            spy.0.lock().unwrap().as_slice(),
            &[("seed".to_string(), "ext".to_string(), "item".to_string(), 3)]
        );
    }
}
