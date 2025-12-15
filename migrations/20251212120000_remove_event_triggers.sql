-- Remove legacy notify triggers now replaced by internal events

-- Thing value insert trigger
DROP TRIGGER IF EXISTS thing_value_insert_notify ON thing_value;
DROP FUNCTION IF EXISTS notify_thing_value_insert();

-- Command insert/update triggers
DROP TRIGGER IF EXISTS thing_command_insert_notify ON thing_command;
DROP TRIGGER IF EXISTS thing_command_update_notify ON thing_command;
DROP FUNCTION IF EXISTS notify_thing_command();

-- Energy reading insert trigger
DROP TRIGGER IF EXISTS energy_reading_insert_notify ON energy_reading;
DROP FUNCTION IF EXISTS notify_energy_reading();

-- User trigger insert trigger
DROP TRIGGER IF EXISTS user_trigger_insert_notify ON user_trigger;
DROP FUNCTION IF EXISTS notify_user_trigger();
