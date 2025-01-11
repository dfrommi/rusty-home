CREATE TABLE item_availability (
    id BIGSERIAL NOT NULL PRIMARY KEY,
    source VARCHAR NOT NULL,
    item VARCHAR NOT NULL,
    last_seen TIMESTAMPTZ NOT NULL,
    marked_offline BOOLEAN NOT NULL,
    considered_offline_after INTERVAL NOT NULL,
    entry_updated TIMESTAMPTZ NOT NULL
);

ALTER TABLE item_availability
    ADD CONSTRAINT item_availability_source_item_key UNIQUE (source, item);

