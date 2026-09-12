-- Machine-bound and privacy-sensitive, so it follows the local_ convention and
-- never syncs. Images live as files; the row holds the path and the byte count
-- so the size bound can be enforced without touching the disk.
CREATE TABLE local_clipboard_history (
    entry_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    text TEXT,
    path TEXT,
    bytes INTEGER NOT NULL,
    -- Identifies the content, so copying the same thing twice reorders the
    -- existing entry instead of adding another.
    fingerprint TEXT NOT NULL UNIQUE,
    copied_at INTEGER NOT NULL
);

CREATE INDEX local_clipboard_history_recent
    ON local_clipboard_history (copied_at DESC);
