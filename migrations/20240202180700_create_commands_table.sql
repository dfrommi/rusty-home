CREATE TABLE THING_COMMANDS (
    id BIGSERIAL NOT NULL PRIMARY KEY,
    type TEXT NOT NULL,
    position TEXT,
    payload JSONB,
    timestamp TIMESTAMPTZ NOT NULL,
    status VARCHAR(50) DEFAULT 'Pending',
    error TEXT
);
