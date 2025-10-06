-- Migrate legacy thing_command source identifiers to current ExternalId values.
-- ExternalId expectations are defined in app/src/home/tests/action.rs.

-- User-triggered HomeKit actions
UPDATE thing_command SET source_type = 'user_trigger_action', source_id = 'homekit::bedroom_ceiling_fan_speed'
  WHERE source_id = 'homekit:Homekit[BedroomCeilingFanSpeed]';
UPDATE thing_command SET source_type = 'user_trigger_action', source_id = 'homekit::dehumidifier_power'
  WHERE source_id = 'homekit:Homekit[DehumidifierPower]';
UPDATE thing_command SET source_type = 'user_trigger_action', source_id = 'homekit::infrared_heater_power'
  WHERE source_id = 'homekit:Homekit[InfraredHeaterPower]';
UPDATE thing_command SET source_type = 'user_trigger_action', source_id = 'homekit::living_room_ceiling_fan_speed'
  WHERE source_id = 'homekit:Homekit[LivingRoomCeilingFanSpeed]';
UPDATE thing_command SET source_type = 'user_trigger_action', source_id = 'homekit::living_room_heating_state'
  WHERE source_id = 'homekit:Homekit[LivingRoomHeatingState]';
UPDATE thing_command SET source_type = 'user_trigger_action', source_id = 'homekit::living_room_tv_energy_saving'
  WHERE source_id = 'homekit:Homekit[LivingRoomTvEnergySaving]';
UPDATE thing_command SET source_type = 'user_trigger_action', source_id = 'homekit::room_of_requirements_heating_state'
  WHERE source_id = 'homekit:Homekit[RoomOfRequirementsHeatingState]';

-- CoolDownWhenOccupied rules
UPDATE thing_command SET source_type = 'cool_down_when_occupied', source_id = 'bedroom_ceiling_fan'
  WHERE source_id = 'planning:CoolDownWhenOccupied[Bedroom]:start';
UPDATE thing_command SET source_type = 'cool_down_when_occupied', source_id = 'living_room_ceiling_fan'
  WHERE source_id = 'planning:CoolDownWhenOccupied[LivingRoom]:start';

-- Dehumidify rule
UPDATE thing_command SET source_type = 'dehumidify', source_id = 'dehumidifier'
  WHERE source_id IN ('planning:Dehumidify:start', 'planning:Dehumidify:stop');

-- FollowDefaultSetting rules
UPDATE thing_command SET source_type = 'follow_default_setting', source_id = 'control_fan::bedroom_ceiling_fan'
  WHERE source_id = 'planning:FollowDefaultSetting[ControlFan[BedroomCeilingFan]]:start';
UPDATE thing_command SET source_type = 'follow_default_setting', source_id = 'control_fan::living_room_ceiling_fan'
  WHERE source_id IN (
    'planning:FollowDefaultSetting[ControlFan[LivingRoomCeilingFan]]:start',
    'planning:FollowDefaultSetting[ControlFan[LivingRoomFan]]:start'
  );
UPDATE thing_command SET source_type = 'follow_default_setting', source_id = 'push_notify::dennis::window_opened'
  WHERE source_id = 'planning:FollowDefaultSetting[PushNotify[WindowOpened - Dennis]]:start';
UPDATE thing_command SET source_type = 'follow_default_setting', source_id = 'push_notify::sabine::window_opened'
  WHERE source_id = 'planning:FollowDefaultSetting[PushNotify[WindowOpened - Sabine]]:start';
UPDATE thing_command SET source_type = 'follow_default_setting', source_id = 'set_energy_saving::living_room_tv'
  WHERE source_id = 'planning:FollowDefaultSetting[SetEnergySaving[LivingRoomTv]]:start';
UPDATE thing_command SET source_type = 'follow_default_setting', source_id = 'set_heating::bathroom'
  WHERE source_id = 'planning:FollowDefaultSetting[SetHeating[Bathroom]]:start';
UPDATE thing_command SET source_type = 'follow_default_setting', source_id = 'set_power::dehumidifier'
  WHERE source_id = 'planning:FollowDefaultSetting[SetPower[Dehumidifier]]:start';
UPDATE thing_command SET source_type = 'follow_default_setting', source_id = 'set_power::infrared_heater'
  WHERE source_id = 'planning:FollowDefaultSetting[SetPower[InfraredHeater]]:start';
UPDATE thing_command SET source_type = 'follow_default_setting', source_id = 'set_power::living_room_notification_light'
  WHERE source_id = 'planning:FollowDefaultSetting[SetPower[LivingRoomNotificationLight]]:start';

-- FollowHeatingSchedule rules
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'bedroom::away'
  WHERE source_id = 'planning:FollowHeatingSchedule[Bedroom - Away]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'bedroom::energy_saving'
  WHERE source_id = 'planning:FollowHeatingSchedule[Bedroom - EnergySaving]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'bedroom::post_ventilation'
  WHERE source_id = 'planning:FollowHeatingSchedule[Bedroom - PostVentilation]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'bedroom::ventilation'
  WHERE source_id = 'planning:FollowHeatingSchedule[Bedroom - Ventilation]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'kitchen::away'
  WHERE source_id = 'planning:FollowHeatingSchedule[Kitchen - Away]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'kitchen::energy_saving'
  WHERE source_id = 'planning:FollowHeatingSchedule[Kitchen - EnergySaving]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'kitchen::post_ventilation'
  WHERE source_id = 'planning:FollowHeatingSchedule[Kitchen - PostVentilation]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'kitchen::ventilation'
  WHERE source_id = 'planning:FollowHeatingSchedule[Kitchen - Ventilation]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'living_room::away'
  WHERE source_id = 'planning:FollowHeatingSchedule[LivingRoom - Away]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'living_room::energy_saving'
  WHERE source_id = 'planning:FollowHeatingSchedule[LivingRoom - EnergySaving]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'living_room::post_ventilation'
  WHERE source_id = 'planning:FollowHeatingSchedule[LivingRoom - PostVentilation]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'living_room::ventilation'
  WHERE source_id = 'planning:FollowHeatingSchedule[LivingRoom - Ventilation]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'room_of_requirements::away'
  WHERE source_id = 'planning:FollowHeatingSchedule[RoomOfRequirements - Away]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'room_of_requirements::energy_saving'
  WHERE source_id = 'planning:FollowHeatingSchedule[RoomOfRequirements - EnergySaving]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'room_of_requirements::post_ventilation'
  WHERE source_id = 'planning:FollowHeatingSchedule[RoomOfRequirements - PostVentilation]:start';
UPDATE thing_command SET source_type = 'follow_heating_schedule', source_id = 'room_of_requirements::ventilation'
  WHERE source_id = 'planning:FollowHeatingSchedule[RoomOfRequirements - Ventilation]:start';

-- InformWindowOpen notifications
UPDATE thing_command SET source_type = 'inform_window_open', source_id = 'push_notification::dennis'
  WHERE source_id = 'planning:InformWindowOpen[Dennis]:start';
UPDATE thing_command SET source_type = 'inform_window_open', source_id = 'push_notification::sabine'
  WHERE source_id = 'planning:InformWindowOpen[Sabine]:start';

DELETE FROM thing_command
  WHERE source_id IN (
    'planning:InformWindowOpen[Dennis]:stop',
    'planning:InformWindowOpen[Sabine]:stop'
  );

-- ProvideAmbientTemperature rules
UPDATE thing_command SET source_type = 'provide_ambient_temperature', source_id = 'thermostat::bedroom'
  WHERE source_id = 'planning:ProvideAmbientTemperature[BedroomThermostat]:start';
UPDATE thing_command SET source_type = 'provide_ambient_temperature', source_id = 'thermostat::kitchen'
  WHERE source_id = 'planning:ProvideAmbientTemperature[KitchenThermostat]:start';
UPDATE thing_command SET source_type = 'provide_ambient_temperature', source_id = 'thermostat::living_room_big'
  WHERE source_id = 'planning:ProvideAmbientTemperature[LivingRoomThermostatBig]:start';
UPDATE thing_command SET source_type = 'provide_ambient_temperature', source_id = 'thermostat::living_room_small'
  WHERE source_id = 'planning:ProvideAmbientTemperature[LivingRoomThermostatSmall]:start';
UPDATE thing_command SET source_type = 'provide_ambient_temperature', source_id = 'thermostat::room_of_requirements'
  WHERE source_id IN (
    'planning:ProvideAmbientTemperature[RoomOfRequirements]:start',
    'planning:ProvideAmbientTemperature[RoomOfRequirementsThermostat]:start'
  );

-- ReduceNoiseAtNight rule
UPDATE thing_command SET source_type = 'reduce_noise_at_night', source_id = 'dehumidifier'
  WHERE source_id = 'planning:ReduceNoiseAtNight:start';

-- SupportVentilationWithFan rules
UPDATE thing_command SET source_type = 'support_ventilation_with_fan', source_id = 'bedroom_ceiling_fan'
  WHERE source_id = 'planning:SupportVentilationWithFan[BedroomCeilingFan]:start';
UPDATE thing_command SET source_type = 'support_ventilation_with_fan', source_id = 'living_room_ceiling_fan'
  WHERE source_id = 'planning:SupportVentilationWithFan[LivingRoomCeilingFan]:start';

-- Remote user trigger
UPDATE thing_command SET source_type = 'user_trigger_action', source_id = 'remote::bedroom_door'
  WHERE source_id = 'remote:Remote[BedroomDoor]';

-- Remove obsolete mqtt entries
DELETE FROM thing_command WHERE source_id = 'mqtt';

-- RequestClosingWindow now maps to InformWindowOpen notification light
UPDATE thing_command SET source_type = 'inform_window_open', source_id = 'notification_light_living_room'
  WHERE source_id = 'planning:RequestClosingWindow:start';

DELETE FROM thing_command WHERE source_id = 'planning:RequestClosingWindow:stop';

-- SaveTvEnergy migrated to FollowDefaultSetting energy-saving command
UPDATE thing_command SET source_type = 'follow_default_setting', source_id = 'set_energy_saving::living_room_tv'
  WHERE source_id = 'planning:SaveTvEnergy:start';
