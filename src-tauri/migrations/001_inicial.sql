CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE chats (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  title      TEXT    NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE messages (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  chat_id    INTEGER NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
  role       TEXT    NOT NULL,
  content    TEXT    NOT NULL,
  sources    TEXT    NOT NULL DEFAULT '[]',
  created_at INTEGER NOT NULL
);

CREATE INDEX idx_messages_chat ON messages(chat_id, id);

CREATE TABLE documents (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  name        TEXT    NOT NULL,
  source_path TEXT    NOT NULL DEFAULT '',
  size_bytes  INTEGER NOT NULL DEFAULT 0,
  embed_model TEXT    NOT NULL,
  created_at  INTEGER NOT NULL
);

CREATE TABLE chunks (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  position    INTEGER NOT NULL,
  content     TEXT    NOT NULL,
  embedding   BLOB    NOT NULL,
  model       TEXT    NOT NULL
);

CREATE INDEX idx_chunks_document ON chunks(document_id);
CREATE INDEX idx_chunks_model ON chunks(model);
