UPDATE thing_command
SET command = command::jsonb - 'until' || '{"duration": "PT4H"}'::jsonb
where command->>'until' is not null;
