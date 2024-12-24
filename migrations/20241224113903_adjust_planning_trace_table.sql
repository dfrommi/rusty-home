CREATE TABLE planning_trace (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    run_id UUID NOT NULL,
    seq INT NOT NULL,
    action VARCHAR NOT NULL,
    goal VARCHAR NOT NULL,
    goal_active BOOLEAN NOT NULL,
    locked BOOLEAN NOT NULL,
    fulfilled BOOLEAN,
    triggered BOOLEAN
);

DROP TABLE IF EXISTS action_plan_log;
