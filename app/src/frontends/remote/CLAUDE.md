# Remote Frontend

Handles remote control triggers from Zigbee2MQTT devices.

Data flow: MQTT (Z2M action events) → Z2mRemoteIncomingDataSource → RemoteService → TriggerClient.

Unrecognized action values are silently skipped; JSON parse errors are logged.
