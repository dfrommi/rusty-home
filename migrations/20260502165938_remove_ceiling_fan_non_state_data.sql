-- Remove non-state data for ceiling fans that no longer exist in code.
-- Keep thing_value and thing_value_tag history intact.

DELETE FROM thing_command
WHERE user_trigger_id IN (
    SELECT id
    FROM user_trigger
    WHERE trigger->>'type' = 'fan_speed'
      AND trigger->>'fan' IN ('living_room_ceiling_fan', 'bedroom_ceiling_fan')
);

DELETE FROM user_trigger
WHERE trigger->>'type' = 'fan_speed'
  AND trigger->>'fan' IN ('living_room_ceiling_fan', 'bedroom_ceiling_fan');
