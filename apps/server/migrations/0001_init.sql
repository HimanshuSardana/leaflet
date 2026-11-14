-- v1 schema. Handlers land later; this migration fixes the structure.
-- ids: UUIDv7 as TEXT (generated in Rust), so nothing leaks insert order.

CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name  TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('admin', 'member')),
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

-- Refresh-token sessions. Only the hash of the token is stored.
CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash      TEXT NOT NULL UNIQUE,
    user_agent      TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_used_at    TEXT,
    expires_at      TEXT NOT NULL,
    revoked_at      TEXT
) STRICT;
CREATE INDEX sessions_user_id_idx ON sessions(user_id);

-- Long-lived bearer tokens for non-browser clients (Android).
CREATE TABLE api_tokens (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    token_hash   TEXT NOT NULL UNIQUE,
    last_used_at TEXT,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    revoked_at   TEXT
) STRICT;
CREATE INDEX api_tokens_user_id_idx ON api_tokens(user_id);

-- One row per uploaded file. No work/edition abstraction in v1.
-- Libraries are per-user: owner_id scopes every book.
CREATE TABLE books (
    id           TEXT PRIMARY KEY,
    owner_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title        TEXT NOT NULL,
    sort_title   TEXT,
    author       TEXT,
    series       TEXT,
    volume       REAL,
    language     TEXT,
    format       TEXT NOT NULL CHECK (format IN ('pdf', 'epub', 'cbz')),
    page_count   INTEGER,
    -- content address: sha256 hex of the original bytes
    sha256       TEXT NOT NULL,
    byte_size    INTEGER NOT NULL,
    cover_path   TEXT,
    added_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (owner_id, sha256)
) STRICT;
CREATE INDEX books_owner_sort_idx ON books(owner_id, sort_title);
CREATE INDEX books_sha256_idx ON books(sha256);

CREATE TABLE tags (
    id      TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name    TEXT NOT NULL COLLATE NOCASE,
    UNIQUE (owner_id, name)
) STRICT;

CREATE TABLE book_tags (
    book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    tag_id  TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (book_id, tag_id)
) STRICT;

CREATE TABLE collections (
    id          TEXT PRIMARY KEY,
    owner_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (owner_id, name)
) STRICT;

CREATE TABLE collection_books (
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    book_id       TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    position      INTEGER,
    added_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (collection_id, book_id)
) STRICT;

-- Progress is the product. locator holds the device-native resume point as
-- JSON ({href, cfi|page|scroll_pct}); fraction is the normalized 0..1 position
-- so a reflowable EPUB and a fixed-layout PDF sort coherently. Device clocks
-- are untrusted: last_modified is the client's view, updated_at is ours.
CREATE TABLE reading_progress (
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id       TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    locator       TEXT NOT NULL DEFAULT '{}',
    fraction      REAL NOT NULL DEFAULT 0 CHECK (fraction >= 0 AND fraction <= 1),
    locator_page  INTEGER,
    status        TEXT NOT NULL DEFAULT 'unread'
                  CHECK (status IN ('unread', 'reading', 'finished', 'dropped')),
    last_modified TEXT,
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (user_id, book_id)
) STRICT;
CREATE INDEX reading_progress_book_idx ON reading_progress(book_id);
