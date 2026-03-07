-- Add optional activation start for user triggers
ALTER TABLE user_trigger
    ADD COLUMN active_from TIMESTAMPTZ;
