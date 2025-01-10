DROP TABLE IF EXISTS planning_trace;

CREATE TABLE planning_trace (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    trace_id VARCHAR,
    steps JSONB NOT NULL
);

