# PCHMD: PC Hardware Monitoring Display
A Cross-Platform GUI and back-end designed to display PC sensor data(ex. Temps, Clocks, etc.) on a separate display (local or external).

## Tentative Planned Features

- [ ] LibreHardwareMonitor as a data source for server backend(Windows, Linux)
- [ ] libsensors (lm-sensors library) as a data source for server backend(Linux)
- [ ] nvml or XNVCtrl as a data source for server backend(Linux)
- [ ] support multiple client gui connections per server backend
- [ ] local client gui running on PC
- [ ] remote client gui running on another PC connected via USB (ex. RPi w/ Display)
- [ ] remote client gui running on another PC connected via Local Network
- [ ] remote client gui running on Android device connected via Local Network (ex. Phone App)
- [ ] configurable gui elements based off client config file
- [ ] configurable data sources based off server config file
- [ ] automatic data source detection and setup on server
