CREATE TABLE action_plan_log (
    id BIGSERIAL PRIMARY KEY,
    run_id UUID NOT NULL,
    seq INT NOT NULL,
    action VARCHAR NOT NULL,
    should_be_started BOOLEAN NOT NULL,
    should_be_stopped BOOLEAN NOT NULL,
    goal_active BOOLEAN NOT NULL,
    locked BOOLEAN NOT NULL,
    fulfilled BOOLEAN,
    running BOOLEAN,
    timestamp TIMESTAMPTZ NOT NULL
);
