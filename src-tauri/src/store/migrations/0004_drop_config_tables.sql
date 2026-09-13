-- Configuration moved to ~/.config/dango/config.json. The preferences and
-- extension_state tables are retired; nothing wrote them, since there was no
-- interface to. Data tables are untouched.
DROP TABLE IF EXISTS preferences;
DROP TABLE IF EXISTS extension_state;
