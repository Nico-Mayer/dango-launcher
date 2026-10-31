//! Frecency: a per-item score from how often and how recently it was launched.
//! Held in memory so ranking never reads the database; launches update memory
//! and persist through a small trait the store implements.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Score halves every 30 days of disuse, so recency decays smoothly while a
/// high launch count still keeps a stale item findable.
pub(super) const HALF_LIFE_MS: f64 = 30.0 * 24.0 * 60.0 * 60.0 * 1000.0;

#[derive(Clone, Copy)]
pub struct Record {
    pub launch_count: u32,
    pub last_launched_at: i64,
}

/// Where frecency is loaded from and saved to. The store implements this; tests
/// use an in-memory stand-in.
pub trait FrecencyPersistence: Send + Sync {
    fn load_all(&self) -> Vec<(String, String, Record)>;
    fn save(&self, extension_id: &str, item_id: &str, record: Record);
}

/// Keyed by extension id, then item id: each extension picks its own item ids.
type ByItem = HashMap<String, HashMap<String, Record>>;

pub struct FrecencyTable {
    records: Mutex<ByItem>,
    persistence: Option<Box<dyn FrecencyPersistence>>,
}

impl FrecencyTable {
    /// Loads existing frecency into memory once at startup.
    pub fn load(persistence: Box<dyn FrecencyPersistence>) -> Self {
        let mut records = ByItem::new();
        for (extension_id, item_id, record) in persistence.load_all() {
            records
                .entry(extension_id)
                .or_default()
                .insert(item_id, record);
        }
        Self {
            records: Mutex::new(records),
            persistence: Some(persistence),
        }
    }

    pub fn in_memory() -> Self {
        Self {
            records: Mutex::new(HashMap::new()),
            persistence: None,
        }
    }

    pub fn score(&self, extension_id: &str, item_id: &str, now: i64) -> f64 {
        let records = self.records.lock().unwrap();
        match records
            .get(extension_id)
            .and_then(|items| items.get(item_id))
        {
            Some(record) => {
                let age = (now - record.last_launched_at).max(0) as f64;
                record.launch_count as f64 * 0.5_f64.powf(age / HALF_LIFE_MS)
            }
            None => 0.0,
        }
    }

    /// Records a launch in memory and persists it. Called off the search path.
    pub fn record_launch(&self, extension_id: &str, item_id: &str, now: i64) {
        let record = {
            let mut records = self.records.lock().unwrap();
            let record = records
                .entry(extension_id.to_string())
                .or_default()
                .entry(item_id.to_string())
                .or_insert(Record {
                    launch_count: 0,
                    last_launched_at: now,
                });
            record.launch_count += 1;
            record.last_launched_at = now;
            *record
        };
        if let Some(persistence) = &self.persistence {
            persistence.save(extension_id, item_id, record);
        }
    }
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl FrecencyPersistence for std::sync::Arc<crate::store::Store> {
    fn load_all(&self) -> Vec<(String, String, Record)> {
        self.load_frecency()
            .unwrap_or_default()
            .into_iter()
            .map(|(extension_id, item_id, launch_count, last_launched_at)| {
                (
                    extension_id,
                    item_id,
                    Record {
                        launch_count,
                        last_launched_at,
                    },
                )
            })
            .collect()
    }

    fn save(&self, extension_id: &str, item_id: &str, record: Record) {
        if let Err(error) = self.save_frecency(
            extension_id,
            item_id,
            record.launch_count,
            record.last_launched_at,
        ) {
            eprintln!("[dango] could not persist frecency for {item_id}: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn an_unknown_item_scores_zero() {
        let table = FrecencyTable::in_memory();
        assert_eq!(table.score("ext", "nope", now_millis()), 0.0);
    }

    #[test]
    fn more_launches_score_higher_at_equal_recency() {
        let table = FrecencyTable::in_memory();
        let now = now_millis();
        table.record_launch("ext", "once", now);
        for _ in 0..5 {
            table.record_launch("ext", "often", now);
        }
        assert!(table.score("ext", "often", now) > table.score("ext", "once", now));
    }

    #[test]
    fn a_stale_launch_decays_below_a_recent_one() {
        let table = FrecencyTable::in_memory();
        let now = now_millis();
        let year = 365 * 24 * 60 * 60 * 1000;
        table.record_launch("ext", "stale", now - year);
        table.record_launch("ext", "fresh", now);
        assert!(table.score("ext", "fresh", now) > table.score("ext", "stale", now));
    }

    #[test]
    fn the_same_item_id_in_two_extensions_scores_apart() {
        let table = FrecencyTable::in_memory();
        let now = now_millis();
        table.record_launch("one", "item", now);
        assert!(table.score("one", "item", now) > 0.0);
        assert_eq!(table.score("two", "item", now), 0.0);
    }

    #[derive(Default)]
    struct Spy {
        saved: Mutex<Vec<(String, String, u32)>>,
    }
    impl FrecencyPersistence for Spy {
        fn load_all(&self) -> Vec<(String, String, Record)> {
            Vec::new()
        }
        fn save(&self, extension_id: &str, item_id: &str, record: Record) {
            self.saved.lock().unwrap().push((
                extension_id.to_string(),
                item_id.to_string(),
                record.launch_count,
            ));
        }
    }

    #[test]
    fn launches_are_persisted() {
        let spy = Arc::new(Spy::default());
        // Box a cloneable handle to the same spy.
        struct Handle(Arc<Spy>);
        impl FrecencyPersistence for Handle {
            fn load_all(&self) -> Vec<(String, String, Record)> {
                self.0.load_all()
            }
            fn save(&self, extension_id: &str, item_id: &str, record: Record) {
                self.0.save(extension_id, item_id, record)
            }
        }
        let table = FrecencyTable::load(Box::new(Handle(spy.clone())));
        table.record_launch("ext", "x", now_millis());
        assert_eq!(
            spy.saved.lock().unwrap().as_slice(),
            &[("ext".into(), "x".into(), 1)]
        );
    }

    #[test]
    fn loads_existing_records_into_memory() {
        struct Seeded;
        impl FrecencyPersistence for Seeded {
            fn load_all(&self) -> Vec<(String, String, Record)> {
                vec![(
                    "ext".into(),
                    "seed".into(),
                    Record {
                        launch_count: 3,
                        last_launched_at: now_millis(),
                    },
                )]
            }
            fn save(&self, _extension_id: &str, _item_id: &str, _record: Record) {}
        }
        let table = FrecencyTable::load(Box::new(Seeded));
        assert!(table.score("ext", "seed", now_millis()) > 0.0);
    }
}
