CREATE TABLE input_history (
    id TEXT PRIMARY KEY,
    input TEXT NOT NULL,
    item_id TEXT NOT NULL,
    use_count INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    UNIQUE (input, item_id)
);
