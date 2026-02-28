-- Audit log: hash-chained, append-only record of every mutation.
-- seq provides an unambiguous total order; prev_hash links entries into a chain.
CREATE TABLE IF NOT EXISTS audit_log (
    seq         BIGSERIAL   NOT NULL,
    id          UUID        PRIMARY KEY,
    timestamp   TIMESTAMPTZ NOT NULL,
    operation   TEXT        NOT NULL,
    entity_type TEXT        NOT NULL,
    entity_id   UUID        NOT NULL,
    data_hash   TEXT        NOT NULL,
    prev_hash   TEXT        NOT NULL,
    entry_hash  TEXT        NOT NULL
);

CREATE INDEX IF NOT EXISTS audit_log_seq_idx       ON audit_log (seq);
CREATE INDEX IF NOT EXISTS audit_log_entity_idx    ON audit_log (entity_id);
CREATE INDEX IF NOT EXISTS audit_log_timestamp_idx ON audit_log (timestamp DESC);
