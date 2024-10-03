CREATE TABLE THING_COMMANDS (
    id BIGSERIAL NOT NULL PRIMARY KEY,
    type TEXT NOT NULL,
    device TEXT,
    payload JSONB,
    timestamp TIMESTAMPTZ NOT NULL,
    status VARCHAR(50),
    error TEXT,
    source VARCHAR(50)
);

