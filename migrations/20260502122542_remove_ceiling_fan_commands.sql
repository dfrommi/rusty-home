-- Remove commands for ceiling fans that no longer exist in code.
DELETE FROM thing_command
WHERE command->>'type' = 'control_fan'
  AND command->>'device' IN ('living_room_ceiling_fan', 'bedroom_ceiling_fan');
