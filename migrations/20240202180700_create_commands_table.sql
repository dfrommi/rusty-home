CREATE TABLE THING_COMMANDS (
    id BIGSERIAL NOT NULL PRIMARY KEY,
    command JSONB NOT NULL,
    created TIMESTAMPTZ NOT NULL,
    status VARCHAR(50) NOT NULL,
    error TEXT,
    source_type VARCHAR(50) NOT NULL,
    source_id VARCHAR(255) NOT NULL
);

CREATE INDEX idx_thing_commands_target ON THING_COMMANDS((command->>'type'),(command->>'device'));
CREATE INDEX idx_thing_commands_created ON THING_COMMANDS(created);

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
AFTER INSERT ON THING_COMMANDS
FOR EACH ROW EXECUTE FUNCTION notify_thing_command('thing_command_insert');

-- Trigger for UPDATE
CREATE TRIGGER thing_command_update_notify
AFTER UPDATE ON THING_COMMANDS
FOR EACH ROW EXECUTE FUNCTION notify_thing_command('thing_command_update');
