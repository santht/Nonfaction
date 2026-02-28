-- Documents table: metadata index for content-addressable files stored on disk.
-- Actual file bytes live on the filesystem; this table tracks hashes and Merkle links.
CREATE TABLE IF NOT EXISTS documents (
    hash             TEXT PRIMARY KEY,
    size_bytes       BIGINT      NOT NULL,
    stored_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    merkle_children  TEXT[]      NOT NULL DEFAULT '{}',
    verified_at      TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS documents_stored_at_idx ON documents (stored_at DESC);
