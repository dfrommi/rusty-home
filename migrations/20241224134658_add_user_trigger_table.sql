CREATE TABLE user_trigger (
    id BIGSERIAL NOT NULL PRIMARY KEY,
    trigger JSONB NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_user_trigger_timestamp ON user_trigger(timestamp);

CREATE OR REPLACE FUNCTION notify_user_trigger() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        TG_ARGV[0],
        json_build_object(
            'id', NEW.id
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for INSERT
CREATE TRIGGER user_trigger_insert_notify
AFTER INSERT ON user_trigger
FOR EACH ROW EXECUTE FUNCTION notify_user_trigger('user_trigger_insert');

