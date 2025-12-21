-- Remove device-state tags/values that no longer exist in code.
WITH removed_tags AS (
    SELECT id
    FROM thing_value_tag
    WHERE (channel = 'raw_vendor_value' AND (name LIKE 'ally_load_estimate::%' OR name LIKE 'ally_load_mean::%'))
        OR (channel = 'temperature' AND name LIKE 'thermostat_external::%')
        OR (channel = 'opened' AND name IN (
            'kitchen_radiator_thermostat',
            'bedroom_radiator_thermostat',
            'living_room_radiator_thermostat_small',
            'living_room_radiator_thermostat_big',
            'room_of_requirements_thermostat',
            'bathroom_thermostat'
        ))
)
DELETE FROM thing_value
WHERE tag_id IN (SELECT id FROM removed_tags);

WITH removed_tags AS (
    SELECT id
    FROM thing_value_tag
    WHERE (channel = 'raw_vendor_value' AND (name LIKE 'ally_load_estimate::%' OR name LIKE 'ally_load_mean::%'))
        OR (channel = 'temperature' AND name LIKE 'thermostat_external::%')
        OR (channel = 'opened' AND name IN (
            'kitchen_radiator_thermostat',
            'bedroom_radiator_thermostat',
            'living_room_radiator_thermostat_small',
            'living_room_radiator_thermostat_big',
            'room_of_requirements_thermostat',
            'bathroom_thermostat'
        ))
)
DELETE FROM thing_value_tag
WHERE id IN (SELECT id FROM removed_tags);

-- Remove commands that were removed from code.
DELETE FROM thing_command
WHERE command->>'type' IN (
    'set_heating',
    'set_thermostat_ambient_temperature',
    'set_thermostat_load_mean'
);
