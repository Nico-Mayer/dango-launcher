-- The user's own content, and the first thing that should exist on both of
-- their machines, so this follows the syncable convention rather than the
-- local_ one the clipboard history uses. Nothing reads updated_at or
-- deleted_at until M9; they are here from the first migration because adding
-- them later is what makes sync impossible.
CREATE TABLE snippets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    template TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER
);

CREATE INDEX snippets_live ON snippets (deleted_at, name);

CREATE TABLE quicklinks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER
);

CREATE INDEX quicklinks_live ON quicklinks (deleted_at, name);
