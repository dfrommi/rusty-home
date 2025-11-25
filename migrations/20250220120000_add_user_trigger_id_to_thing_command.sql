-- Add optional linkage to the originating user_trigger entry for commands.
ALTER TABLE thing_command
    ADD COLUMN user_trigger_id BIGINT;
