CREATE TABLE ENERGY_READING (
    id BIGSERIAL PRIMARY KEY,
    type TEXT NOT NULL,
    name TEXT NOT NULL,
    value FLOAT8 NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL
);
