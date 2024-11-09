CREATE TABLE energy_reading (
    id BIGSERIAL PRIMARY KEY,
    type VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    value FLOAT8 NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL
);
