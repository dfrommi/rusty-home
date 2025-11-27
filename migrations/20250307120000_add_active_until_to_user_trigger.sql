-- Add optional expiry window for user triggers
ALTER TABLE user_trigger
    ADD COLUMN active_until TIMESTAMPTZ;

-- Populate existing rows with a one-hour active window
UPDATE user_trigger
SET active_until = timestamp + INTERVAL '1 hour'
WHERE active_until IS NULL;
