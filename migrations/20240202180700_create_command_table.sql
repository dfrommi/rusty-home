CREATE TABLE thing_command (
    id BIGSERIAL NOT NULL PRIMARY KEY,
    command JSONB NOT NULL,
    created TIMESTAMPTZ NOT NULL,
    status VARCHAR NOT NULL,
    error TEXT,
    source_type VARCHAR NOT NULL,
    source_id VARCHAR NOT NULL
);

CREATE INDEX idx_thing_command_target ON thing_command((command->>'type'),(command->>'device'));
CREATE INDEX idx_thing_command_created ON thing_command(created);

CREATE OR REPLACE FUNCTION notify_thing_command() RETURNS TRIGGER AS $$
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
CREATE TRIGGER thing_command_insert_notify
AFTER INSERT ON thing_command
FOR EACH ROW EXECUTE FUNCTION notify_thing_command('thing_command_insert');

-- Trigger for UPDATE
CREATE TRIGGER thing_command_update_notify
AFTER UPDATE ON thing_command
FOR EACH ROW EXECUTE FUNCTION notify_thing_command('thing_command_update');
