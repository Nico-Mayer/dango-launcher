-- Rows from before this migration keep NULL: no source and no size is shown.
ALTER TABLE local_clipboard_history ADD COLUMN source TEXT;
ALTER TABLE local_clipboard_history ADD COLUMN width INTEGER;
ALTER TABLE local_clipboard_history ADD COLUMN height INTEGER;
