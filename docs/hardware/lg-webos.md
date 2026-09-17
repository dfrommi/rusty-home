# LG WebOS

Model: OLED65C8LLA
Firmware Version: 5.50.70

## HomeAssistant Limitations

Energy-saving mode can only be turned on/off via an alert-based hack with some precise auto-clicking buttons.
Reading back the energy samvin mode is not possible in any way via the official interfaces.

## Rooting the TV

Root-access could be gained via [dejavuln-autoroot](https://github.com/throwaway96/dejavuln-autoroot) approach.
Afterwards, telnet access is available:

```
nc tv-ip 23
```

Getting the energy-saving value:

```
luna-send -n 1 luna://com.webos.settingsservice/getSystemSettings '{"category":"picture","keys":["energySaving"]}'
```

## MQTT Bridge

<https://github.com/rorygallagher2024/lg-webos-mqtt/>
