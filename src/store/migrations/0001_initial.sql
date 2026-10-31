CREATE TABLE extension_state (
    id TEXT PRIMARY KEY,
    extension_id TEXT NOT NULL UNIQUE,
    enabled INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER
);

CREATE TABLE preferences (
    id TEXT PRIMARY KEY,
    extension_id TEXT NOT NULL,
    command_id TEXT,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER
);

CREATE UNIQUE INDEX preferences_scope
    ON preferences (extension_id, coalesce(command_id, ''), key);

CREATE TABLE frecency (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL UNIQUE,
    launch_count INTEGER NOT NULL DEFAULT 0,
    last_launched_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER
);

-- local_ tables are machine-bound and rebuilt from the OS; they never sync,
-- so they skip the id/updated_at/deleted_at convention on purpose.
CREATE TABLE local_app_index (
    app_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    target TEXT,
    icon TEXT,
    indexed_at INTEGER NOT NULL
);
