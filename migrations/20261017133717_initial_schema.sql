-- Initial schema:
-- * Timestamps are INTEGER Unix epoch seconds, UTC.
-- * On every connection `PRAGMA foreign_keys = ON;` should be ran.
-- * Indexes are not defined yet except for the ones related to folder names.

-- Folders can be nested.
CREATE TABLE folders (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id INTEGER REFERENCES folders(id) ON DELETE CASCADE -- NULL represents a top-level folder
) STRICT;

-- Folder names must be unique among root-level folders.
CREATE UNIQUE INDEX uq_folders_root_name
    ON folders(name)
    WHERE parent_id IS NULL;

-- Folder names must be unique among sibling folders.
CREATE UNIQUE INDEX uq_folders_sibling_name
    ON folders(parent_id, name)
    WHERE parent_id IS NOT NULL;

CREATE TABLE feeds (
    id INTEGER PRIMARY KEY,
    url TEXT NOT NULL UNIQUE,
    title TEXT,
    site_url TEXT,
    favicon BLOB, -- Cached favicon image bytes.
    folder_id INTEGER REFERENCES folders(id) ON DELETE SET NULL,

   -- HTTP cache validators.
    etag TEXT,
    http_last_modified TEXT,

    created_at INTEGER NOT NULL DEFAULT (unixepoch()), -- When user subscribed.

    last_attempt_at INTEGER, -- Most recent refresh attempt.
    last_fetched_at INTEGER, -- Most recent successful refresh attempt.
    last_error TEXT -- Most recent error, cleared after a success.
) STRICT;

CREATE TABLE articles (
    id INTEGER PRIMARY KEY,
    guid TEXT NOT NULL, -- May be absent or unreliable.

    -- Each item belongs to a feed.
    feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,

    url TEXT, -- Nullable: URL may appear on multiple feeds.
    title TEXT,
    author TEXT,
    summary TEXT,
    content TEXT,

    published_at INTEGER, -- Published provided timestamp; may be unknown.
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    last_fetched_at INTEGER,

    read INTEGER NOT NULL DEFAULT 0 CHECK (read IN (0, 1)),
    saved INTEGER NOT NULL DEFAULT 0 CHECK (saved IN (0, 1)),

    UNIQUE (feed_id, guid)
) STRICT;
