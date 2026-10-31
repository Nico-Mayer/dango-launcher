-- Rows were keyed by item id alone and cannot be told apart by extension, so
-- they are dropped rather than migrated.
DROP TABLE frecency;
DROP TABLE input_history;

CREATE TABLE frecency (
    id TEXT PRIMARY KEY,
    extension_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    launch_count INTEGER NOT NULL DEFAULT 0,
    last_launched_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    UNIQUE (extension_id, item_id)
);

CREATE TABLE input_history (
    id TEXT PRIMARY KEY,
    input TEXT NOT NULL,
    extension_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    use_count INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    UNIQUE (input, extension_id, item_id)
);
