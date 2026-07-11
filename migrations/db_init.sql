CREATE TABLE clipboard_entries (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    content_type TEXT NOT NULL CHECK(content_type IN ('text', 'url', 'image')),
    text_content TEXT,
    blob_content BLOB,
    created_at  INTEGER NOT NULL,
    size_bytes  INTEGER NOT NULL
);

CREATE INDEX idx_created_at ON clipboard_entries(created_at DESC);
