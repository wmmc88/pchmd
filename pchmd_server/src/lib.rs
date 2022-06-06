#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]

extern crate core;

use std::collections::HashMap;
use std::fmt;
use std::io::{stdout, Write};
use std::time::{Duration, Instant};
use std::{error::Error, net::SocketAddr, sync::Arc};

use capnp::serialize;
use crossterm::{ExecutableCommand, QueueableCommand};
use futures_util::StreamExt;
use lm_sensors::prelude::*;
use lm_sensors::value::Unit;
use lm_sensors::Value;
use quinn::{Endpoint, IncomingUniStreams};
use rand::prelude::*;
use rand_pcg::Pcg64Mcg;
use rand_seeder::Seeder;
use tabled::Tabled;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

pub mod pchmd_capnp {
    #![allow(clippy::all)]
    #![allow(clippy::pedantic)]
    #![allow(clippy::nursery)]
    #![allow(clippy::cargo)]
    include!(concat!(env!("OUT_DIR"), "/pchmd_capnp.rs"));
}

const DEFAULT_STALE_TIME_SECONDS: f64 = 1.0;

// The following values were determined using ewma.ipynb
// TODO: add way to configure(config file, new constructor, builder pattern, etc.) these values:
const DEFAULT_EWMA_ALPHA_VALUE: f64 = 0.3;
const DEFAULT_UPDATE_PERIOD_SECONDS: f64 = 0.2;

pub struct Server {
    data_sources: Vec<DataSource>,
    transport_interfaces: Vec<TransportInterface>,
    sensor_data: SensorDataMap,

    stale_time_seconds: f64,
    ewma_alpha_value: f64, // alpha value used for exponentially weighted moving average of each SensorValue
    update_period_seconds: f64,
}

impl Server {
    #[must_use]
    pub fn new(
        data_sources: Vec<DataSource>,
        transport_interfaces: Vec<TransportInterface>,
    ) -> Self {
        Self {
            data_sources,
            transport_interfaces,
            sensor_data: SensorDataMap::new(),
            stale_time_seconds: DEFAULT_STALE_TIME_SECONDS,
            ewma_alpha_value: DEFAULT_EWMA_ALPHA_VALUE,
            update_period_seconds: DEFAULT_UPDATE_PERIOD_SECONDS,
        }
    }

    pub fn run(&mut self) {
        let update_period_duration = Duration::from_secs_f64(self.update_period_seconds);
        let mut last_start_time = Instant::now();
        loop {
            let elapsed_duration = last_start_time.elapsed();
            if elapsed_duration < update_period_duration {
                std::thread::sleep(update_period_duration - last_start_time.elapsed());
            } else if elapsed_duration > update_period_duration * 2 {
                // TODO: add logging with time
                eprintln!(
                    "Update loop period exceeded by {:#?}",
                    elapsed_duration - update_period_duration
                );
            }
            last_start_time = Instant::now();
            self.run_once();
        }
    }

    fn run_once(&mut self) {
        // TODO: evaluate running each data source in parallel (queues to a thread that manages the data?)
        for data_source in &self.data_sources {
            data_source.update_values(&mut self.sensor_data, self.ewma_alpha_value);
        }

        let serialized_msg = self.serialize_to_capnproto();

        for interface in &self.transport_interfaces {
            interface.send_message(serialized_msg.clone());
        }
    }

    fn serialize_to_capnproto(&self) -> Arc<Vec<u8>> {
        let mut message = capnp::message::Builder::new_default();
        {
            let mut computer_info = message.init_root::<pchmd_capnp::computer_info::Builder>();
            computer_info.set_name("My Gaming PC"); //todo: get from hostname

            // Seed uuid from mac-address so that it matches whether running in any OS
            // TODO: prefer using eth0 mac
            let mac_address = mac_address::get_mac_address().unwrap().unwrap().to_string();
            let uuid = uuid::Builder::from_random_bytes(
                Seeder::from(mac_address).make_rng::<Pcg64Mcg>().gen(),
            )
            .into_uuid();
            let (upper, lower) = uuid.as_u64_pair();
            computer_info.set_uuid_upper(upper);
            computer_info.set_uuid_lower(lower);

            computer_info.set_operating_system(std::env::consts::OS);

            let mut sensors = computer_info.init_sensors(self.sensor_data.len() as u32);
            for (index, (sensor_data_key, sensor_data_value)) in self.sensor_data.iter().enumerate()
            {
                let mut sensor_data = sensors.reborrow().get(index as u32);
                sensor_data.set_sensor_name(sensor_data_key.sensor_name.as_str());
                sensor_data.set_data_source_name(sensor_data_key.data_source_name.as_str());

                let mut current = sensor_data.reborrow().init_current();
                {
                    match &sensor_data_value.current_value {
                        SensorValue::Float(value) => {
                            current.set_float_value(*value);
                        }
                        SensorValue::RawBool(value) => {
                            current.set_bool_value(value.round() as u8 != 0);
                        }
                        SensorValue::Text(value) => {
                            current.set_string_value(value.as_str());
                        }
                        SensorValue::Bool(_) => {
                            unreachable!();
                        }
                    }
                }
                let mut average = sensor_data.reborrow().init_average();
                {
                    match &sensor_data_value.average_value {
                        SensorValue::Float(value) => {
                            average.set_float_value(*value);
                        }
                        SensorValue::RawBool(value) => {
                            average.set_bool_value(value.round() as u8 != 0);
                        }
                        SensorValue::Text(value) => {
                            average.set_string_value(value.as_str());
                        }
                        SensorValue::Bool(_) => {
                            unreachable!();
                        }
                    }
                }
                let mut minimum = sensor_data.reborrow().init_minimum();
                {
                    match &sensor_data_value.minimum_value {
                        SensorValue::Float(value) => {
                            minimum.set_float_value(*value);
                        }
                        SensorValue::RawBool(value) => {
                            minimum.set_bool_value(value.round() as u8 != 0);
                        }
                        SensorValue::Text(value) => {
                            minimum.set_string_value(value.as_str());
                        }
                        SensorValue::Bool(_) => {
                            unreachable!();
                        }
                    }
                }
                let mut maximum = sensor_data.reborrow().init_maximum();
                {
                    match &sensor_data_value.maximum_value {
                        SensorValue::Float(value) => {
                            maximum.set_float_value(*value);
                        }
                        SensorValue::RawBool(value) => {
                            maximum.set_bool_value(value.round() as u8 != 0);
                        }
                        SensorValue::Text(value) => {
                            maximum.set_string_value(value.as_str());
                        }
                        SensorValue::Bool(_) => {
                            unreachable!();
                        }
                    }
                }

                if let Some(measurement_unit) = &sensor_data_value.measurement_unit {
                    match measurement_unit {
                        MeasurementUnit::Volt => {
                            sensor_data.reborrow().set_measurement_unit(
                                pchmd_capnp::sensor_data::MeasurementUnit::Volt,
                            );
                        }
                        MeasurementUnit::Amp => {
                            sensor_data.reborrow().set_measurement_unit(
                                pchmd_capnp::sensor_data::MeasurementUnit::Amp,
                            );
                        }
                        MeasurementUnit::Watt => {
                            sensor_data.reborrow().set_measurement_unit(
                                pchmd_capnp::sensor_data::MeasurementUnit::Watt,
                            );
                        }
                        MeasurementUnit::Joule => {
                            sensor_data.reborrow().set_measurement_unit(
                                pchmd_capnp::sensor_data::MeasurementUnit::Joule,
                            );
                        }
                        MeasurementUnit::Celcius => {
                            sensor_data.reborrow().set_measurement_unit(
                                pchmd_capnp::sensor_data::MeasurementUnit::Celcius,
                            );
                        }
                        MeasurementUnit::Second => {
                            sensor_data.reborrow().set_measurement_unit(
                                pchmd_capnp::sensor_data::MeasurementUnit::Second,
                            );
                        }
                        MeasurementUnit::RotationPerMinute => {
                            sensor_data.reborrow().set_measurement_unit(
                                pchmd_capnp::sensor_data::MeasurementUnit::RotationPerMinute,
                            );
                        }
                        MeasurementUnit::Percentage => {
                            sensor_data.reborrow().set_measurement_unit(
                                pchmd_capnp::sensor_data::MeasurementUnit::Percentage,
                            );
                        }
                    }
                } else {
                    sensor_data
                        .reborrow()
                        .set_measurement_unit(pchmd_capnp::sensor_data::MeasurementUnit::None);
                }

                if sensor_data_value.last_update_instant.elapsed()
                    > Duration::from_secs_f64(self.stale_time_seconds)
                {
                    sensor_data.set_is_stale(true);
                } else {
                    sensor_data.set_is_stale(false);
                }
            }
        }
        let mut buffer = Vec::new();
        serialize::write_message(&mut buffer, &message).unwrap();
        Arc::new(buffer)
    }
}

#[derive(Debug)]
pub enum DataSource {
    Libsensors(LibsensorsDataSource),
}

impl DataSource {
    const fn get_name(&self) -> &str {
        match self {
            DataSource::Libsensors(_) => LibsensorsDataSource::NAME,
        }
    }

    fn get_version(&self) -> Option<&str> {
        match self {
            DataSource::Libsensors(libsensors_data_source) => libsensors_data_source.get_version(),
        }
    }

    fn update_values(&self, sensor_data_map: &mut SensorDataMap, ewma_alpha_value: f64) {
        match self {
            DataSource::Libsensors(data_source) => {
                data_source.update_values(sensor_data_map, ewma_alpha_value);
            }
        }
    }
}

type SensorDataMap = HashMap<SensorDataKey, SensorData>;

#[derive(Debug, Eq, Hash, PartialEq)]
struct SensorDataKey {
    sensor_name: String,
    data_source_name: String,
}

#[derive(Debug)]
struct SensorData {
    current_value: SensorValue,
    average_value: SensorValue,
    minimum_value: SensorValue,
    maximum_value: SensorValue,

    measurement_unit: Option<MeasurementUnit>,

    last_update_instant: Instant,
}

#[derive(Debug, Clone)]
enum SensorValue {
    Float(f64),
    Bool(bool),
    Text(String),

    RawBool(f64), // f64 as type to be able to min/max/average subsequent values
}

impl fmt::Display for SensorValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
enum MeasurementUnit {
    Volt,
    Amp,
    Watt,
    Joule,
    Celcius,
    Second,
    RotationPerMinute,
    Percentage,
}

#[derive(Default, Debug)]
pub struct LibsensorsDataSource {
    lm_sensors_handle: Option<Box<lm_sensors::LMSensors>>,
    version: Option<String>,
}

impl<'a> LibsensorsDataSource {
    const NAME: &'a str = "libsensors (lm-sensors library)";

    #[must_use]
    pub fn new() -> Self {
        let mut libsensors_data_source = Self::default();
        libsensors_data_source.init();
        libsensors_data_source
    }

    fn init(&mut self) {
        let lm_sensors_handle = lm_sensors::Initializer::default().initialize();
        if let Err(error) = lm_sensors_handle {
            panic!("Failed to initialize LibsensorsDataSource with error: {error}");
        }

        self.lm_sensors_handle = Some(Box::new(lm_sensors_handle.unwrap()));
        self.version = self
            .lm_sensors_handle
            .as_ref()
            .unwrap()
            .version()
            .map(str::to_string);
    }

    fn get_version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    fn update_values(&self, sensor_data_map: &mut SensorDataMap, ewma_alpha_value: f64) {
        for chip in self.lm_sensors_handle.as_ref().unwrap().chip_iter(None) {
            let sensor_location = chip.path().map_or_else(
                || format!("{} at ({})", chip, chip.bus()),
                |path| format!("{} at ({} [{}])", chip, chip.bus(), path.display()),
            );

            for feature in chip.feature_iter() {
                let sensor_feature_name = if let Some(Ok(feature_name)) = feature.name() {
                    format!("{}[{}]", feature, feature_name)
                } else {
                    format!("{}", feature)
                };

                for sub_feature in feature.sub_feature_iter() {
                    let full_sensor_name =
                        format!("{sub_feature} from {sensor_feature_name} on {sensor_location}");
                    if let Ok(value) = sub_feature.value() {
                        if let Ok(sensor_value) = Self::get_value(&value) {
                            let sensor_data_map_key = SensorDataKey {
                                sensor_name: full_sensor_name,
                                data_source_name: Self::NAME.to_string(),
                            };

                            if let Some(sensor_data) = sensor_data_map.get_mut(&sensor_data_map_key)
                            {
                                sensor_data.current_value = sensor_value.clone();

                                {
                                    let sensor_value = sensor_value.clone();
                                    //sensor_data.average_value =
                                    match &sensor_data.average_value {
                                        SensorValue::Float(average_value) => {
                                            if let SensorValue::Float(current_value) = sensor_value
                                            {
                                                let average_value = ewma_alpha_value
                                                    * current_value
                                                    + (1.0 - ewma_alpha_value) * average_value;
                                                sensor_data.average_value =
                                                    SensorValue::Float(average_value);
                                            }
                                        }
                                        SensorValue::RawBool(average_value) => {
                                            if let SensorValue::RawBool(current_value) =
                                                sensor_value
                                            {
                                                let average_value = ewma_alpha_value
                                                    * current_value
                                                    + (1.0 - ewma_alpha_value) * average_value;
                                                sensor_data.average_value =
                                                    SensorValue::Float(average_value);
                                            }
                                        }
                                        SensorValue::Text(_average_value) => {
                                            // TODO: should have a count and set average value to highest count
                                        }
                                        SensorValue::Bool(_) => {
                                            unreachable!();
                                        }
                                    }
                                }
                                {
                                    let sensor_value = sensor_value.clone();
                                    match &sensor_data.minimum_value {
                                        SensorValue::Float(minimum_value) => {
                                            if let SensorValue::Float(current_value) = sensor_value
                                            {
                                                if current_value < *minimum_value {
                                                    sensor_data.minimum_value = sensor_value;
                                                }
                                            }
                                        }
                                        SensorValue::RawBool(minimum_value) => {
                                            if let SensorValue::RawBool(current_value) =
                                                sensor_value
                                            {
                                                if current_value < *minimum_value {
                                                    sensor_data.minimum_value = sensor_value;
                                                }
                                            }
                                        }
                                        SensorValue::Text(minimum_value) => {
                                            if let SensorValue::Text(ref current_value) =
                                                sensor_value
                                            {
                                                if *current_value < *minimum_value {
                                                    sensor_data.minimum_value = sensor_value;
                                                }
                                            }
                                        }
                                        SensorValue::Bool(_) => {
                                            unreachable!();
                                        }
                                    }
                                }
                                match &sensor_data.maximum_value {
                                    SensorValue::Float(maximum_value) => {
                                        if let SensorValue::Float(current_value) = sensor_value {
                                            if current_value > *maximum_value {
                                                sensor_data.maximum_value = sensor_value;
                                            }
                                        }
                                    }
                                    SensorValue::RawBool(maximum_value) => {
                                        if let SensorValue::RawBool(current_value) = sensor_value {
                                            if current_value > *maximum_value {
                                                sensor_data.maximum_value = sensor_value;
                                            }
                                        }
                                    }
                                    SensorValue::Text(maximum_value) => {
                                        if let SensorValue::Text(ref current_value) = sensor_value {
                                            if *current_value > *maximum_value {
                                                sensor_data.maximum_value = sensor_value;
                                            }
                                        }
                                    }
                                    SensorValue::Bool(_) => {
                                        unreachable!();
                                    }
                                }

                                sensor_data.last_update_instant = Instant::now();
                            } else {
                                sensor_data_map.insert(
                                    sensor_data_map_key,
                                    SensorData {
                                        current_value: sensor_value.clone(),
                                        average_value: sensor_value.clone(),
                                        minimum_value: sensor_value.clone(),
                                        maximum_value: sensor_value,
                                        measurement_unit: Self::get_measurement_unit(&value),
                                        last_update_instant: Instant::now(),
                                    },
                                );
                            }
                        }
                    } else {
                        eprintln!("Failed to get value for {full_sensor_name}!");
                    }
                }
            }
        }
    }

    fn get_value(value: &Value) -> Result<SensorValue, ()> {
        match value {
            Value::VoltageInput(value)
            | Value::VoltageMinimum(value)
            | Value::VoltageMaximum(value)
            | Value::VoltageLCritical(value)
            | Value::VoltageCritical(value)
            | Value::VoltageAverage(value)
            | Value::VoltageLowest(value)
            | Value::VoltageHighest(value)
            | Value::FanInput(value)
            | Value::FanMinimum(value)
            | Value::FanMaximum(value)
            | Value::FanDivisor(value)
            | Value::FanPulses(value)
            | Value::TemperatureInput(value)
            | Value::TemperatureMaximum(value)
            | Value::TemperatureMaximumHysteresis(value)
            | Value::TemperatureMinimum(value)
            | Value::TemperatureCritical(value)
            | Value::TemperatureCriticalHysteresis(value)
            | Value::TemperatureLCritical(value)
            | Value::TemperatureEmergency(value)
            | Value::TemperatureEmergencyHysteresis(value)
            | Value::TemperatureLowest(value)
            | Value::TemperatureHighest(value)
            | Value::TemperatureMinimumHysteresis(value)
            | Value::TemperatureLCriticalHysteresis(value)
            | Value::TemperatureOffset(value)
            | Value::PowerAverage(value)
            | Value::PowerAverageHighest(value)
            | Value::PowerAverageLowest(value)
            | Value::PowerInput(value)
            | Value::PowerInputHighest(value)
            | Value::PowerInputLowest(value)
            | Value::PowerCap(value)
            | Value::PowerCapHysteresis(value)
            | Value::PowerMaximum(value)
            | Value::PowerCritical(value)
            | Value::PowerMinimum(value)
            | Value::PowerLCritical(value)
            | Value::PowerAverageInterval(value)
            | Value::EnergyInput(value)
            | Value::CurrentInput(value)
            | Value::CurrentMinimum(value)
            | Value::CurrentMaximum(value)
            | Value::CurrentLCritical(value)
            | Value::CurrentCritical(value)
            | Value::CurrentAverage(value)
            | Value::CurrentLowest(value)
            | Value::CurrentHighest(value)
            | Value::HumidityInput(value)
            | Value::VoltageID(value) => Ok(SensorValue::Float(*value)),

            Value::VoltageAlarm(value)
            | Value::VoltageMinimumAlarm(value)
            | Value::VoltageMaximumAlarm(value)
            | Value::VoltageBeep(value)
            | Value::VoltageLCriticalAlarm(value)
            | Value::VoltageCriticalAlarm(value)
            | Value::FanAlarm(value)
            | Value::FanFault(value)
            | Value::FanBeep(value)
            | Value::FanMinimumAlarm(value)
            | Value::FanMaximumAlarm(value)
            | Value::TemperatureAlarm(value)
            | Value::TemperatureMaximumAlarm(value)
            | Value::TemperatureMinimumAlarm(value)
            | Value::TemperatureCriticalAlarm(value)
            | Value::TemperatureFault(value)
            | Value::TemperatureBeep(value)
            | Value::TemperatureEmergencyAlarm(value)
            | Value::TemperatureLCriticalAlarm(value)
            | Value::PowerAlarm(value)
            | Value::PowerCapAlarm(value)
            | Value::PowerMaximumAlarm(value)
            | Value::PowerCriticalAlarm(value)
            | Value::PowerMinimumAlarm(value)
            | Value::PowerLCriticalAlarm(value)
            | Value::CurrentAlarm(value)
            | Value::CurrentMinimumAlarm(value)
            | Value::CurrentMaximumAlarm(value)
            | Value::CurrentBeep(value)
            | Value::CurrentLCriticalAlarm(value)
            | Value::CurrentCriticalAlarm(value)
            | Value::IntrusionAlarm(value)
            | Value::IntrusionBeep(value)
            | Value::BeepEnable(value) => Ok(SensorValue::RawBool(f64::from(i8::from(*value)))),

            Value::TemperatureType(value) => Ok(SensorValue::Text(value.to_string())),

            Value::Unknown { kind, value } => {
                eprintln!(
                    "Encountered unknown value type in libsensors. Value: {value} Kind: {kind}"
                );
                Ok(SensorValue::Float(*value))
            }
            unknown_value_type => {
                eprintln!(
                    "Encountered unhandled unknown value type in libsensors: {unknown_value_type}"
                );
                Err(())
            }
        }
    }

    fn get_measurement_unit(value: &Value) -> Option<MeasurementUnit> {
        match value.unit() {
            Unit::None => None,
            Unit::Volt => Some(MeasurementUnit::Volt),
            Unit::Amp => Some(MeasurementUnit::Amp),
            Unit::Watt => Some(MeasurementUnit::Watt),
            Unit::Joule => Some(MeasurementUnit::Joule),
            Unit::Celcius => Some(MeasurementUnit::Celcius),
            Unit::Second => Some(MeasurementUnit::Second),
            Unit::RotationPerMinute => Some(MeasurementUnit::RotationPerMinute),
            Unit::Percentage => Some(MeasurementUnit::Percentage),
            unknown_measurement_unit => {
                eprintln!("Encountered unknown measurement unit in libsensors: {unknown_measurement_unit}");
                None
            }
        }
    }
}

#[derive(Debug)]
pub enum TransportInterface {
    QUIC(QUICTransportInterface),
}

impl TransportInterface {
    fn send_message(&self, serialized_message: Arc<Vec<u8>>) {
        match self {
            TransportInterface::QUIC(transport_interface) => {
                transport_interface.send_message(serialized_message);
            }
        }
    }
}

#[derive(Debug)]
pub struct QUICTransportInterface {
    serialized_sensor_data_sender: broadcast::Sender<Arc<Vec<u8>>>,
    _tokio_runtime: tokio::runtime::Runtime, // must keep runtime in scope to keep quic_server_task_handle and other spawned tasks alive
    quic_server_task_handle: tokio::task::JoinHandle<()>,
}

impl<'a> QUICTransportInterface {
    const CERT_PEM: &'a str = "-----BEGIN CERTIFICATE-----
MIIBUjCB+aADAgECAgkAz1vuzG6opxQwCgYIKoZIzj0EAwIwITEfMB0GA1UEAwwW
cmNnZW4gc2VsZiBzaWduZWQgY2VydDAgFw03NTAxMDEwMDAwMDBaGA80MDk2MDEw
MTAwMDAwMFowITEfMB0GA1UEAwwWcmNnZW4gc2VsZiBzaWduZWQgY2VydDBZMBMG
ByqGSM49AgEGCCqGSM49AwEHA0IABJHCy1MLoykAXS8sD1jXBDfpNeVzZAGJJ8Fv
Tu/7OrYj4kEomKbl0qn4uYK/wmEgPwjDoCe+2vg8FJTDT28txGSjGDAWMBQGA1Ud
EQQNMAuCCWxvY2FsaG9zdDAKBggqhkjOPQQDAgNIADBFAiBUOY7QT2ZUocjbt35I
f9C3ificV0wk6hvrp6sY4UQUTgIhAJFLMmmBC3o9NOJNaWpdTuUKFIzQZFl61gFu
MWcWnCgP
-----END CERTIFICATE-----";

    const CERT_PRIVATE_KEY: &'a str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgDdLdN7LoG7QQ5yk4
ufdOECGHysL92BBCRPN9xoV/qTmhRANCAASRwstTC6MpAF0vLA9Y1wQ36TXlc2QB
iSfBb07v+zq2I+JBKJim5dKp+LmCv8JhID8Iw6Anvtr4PBSUw09vLcRk
-----END PRIVATE KEY-----";

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    async fn quic_server_task(sender: broadcast::Sender<Arc<Vec<u8>>>) {
        let server_addr = "127.0.0.1:5000".parse().unwrap(); // TODO: configurable
        let mut incoming = Self::make_server_endpoint(server_addr).unwrap();

        while let Some(conn) = incoming.next().await {
            let new_connection = conn.await.unwrap();
            println!(
                "[server] connection accepted: addr={}",
                new_connection.connection.remote_address()
            );
            loop {
                // TODO: exit condition?
                let send_stream = new_connection.connection.open_uni().await.unwrap();
                tokio::spawn(Self::quic_stream_task(send_stream, sender.subscribe()));
            }
        }

        // TODO: graceful shutdown of connection tasks
        unreachable!("QUIC SERVER TASK ENDED PREMPTIVELY");
    }

    async fn quic_stream_task(
        mut send_stream: quinn::SendStream,
        mut receiver: broadcast::Receiver<Arc<Vec<u8>>>,
    ) {
        match receiver.recv().await {
            Ok(serialized_message) => {
                send_stream
                    .write_all(serialized_message.as_slice())
                    .await
                    .unwrap();
            }
            Err(RecvError::Closed) => {
                eprintln!("Server Closing?!!!!!!!"); // TODO: better error message
            }
            Err(RecvError::Lagged(_)) => {
                eprintln!("Lagged!!!!!!!"); // TODO: better error message
            }
        }
        send_stream.finish().await.unwrap();
    }

    fn make_server_endpoint(bind_addr: SocketAddr) -> Result<quinn::Incoming, Box<dyn Error>> {
        let cert_der = match rustls_pemfile::read_one(&mut Self::CERT_PEM.as_bytes())
            .unwrap()
            .unwrap()
        {
            rustls_pemfile::Item::X509Certificate(cert_der) => cert_der,
            _ => {
                unreachable!()
            }
        };
        let cert_chain = vec![rustls::Certificate(cert_der)];

        let private_key_der = match rustls_pemfile::read_one(&mut Self::CERT_PRIVATE_KEY.as_bytes())
            .unwrap()
            .unwrap()
        {
            rustls_pemfile::Item::PKCS8Key(private_key_der) => private_key_der,

            _ => {
                unreachable!()
            }
        };
        let private_key = rustls::PrivateKey(private_key_der);

        let server_config = quinn::ServerConfig::with_single_cert(cert_chain, private_key)?;
        let (_endpoint, incoming) = Endpoint::server(server_config, bind_addr)?;
        Ok(incoming)
    }

    fn send_message(&self, serialized_message: Arc<Vec<u8>>) {
        match self.serialized_sensor_data_sender.send(serialized_message) {
            Ok(_) => {}
            Err(_) => {
                eprintln!("No QUICTransportInterface clients to send data to.");
            }
        }
    }
}

impl Default for QUICTransportInterface {
    fn default() -> Self {
        let (serialized_sensor_data_sender, _serialized_sensor_data_receiver) =
            broadcast::channel((2.0 / DEFAULT_UPDATE_PERIOD_SECONDS).ceil() as usize); // todo: configurable capacity?

        let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let quic_server_task_handle = tokio_runtime.spawn(Self::quic_server_task(
            serialized_sensor_data_sender.clone(),
        ));
        // TODO: graceful shutdown of quic_server_task_handle

        Self {
            serialized_sensor_data_sender,
            _tokio_runtime: tokio_runtime,
            quic_server_task_handle,
        }
    }
}

/// client code
pub struct CLIClient {}

impl<'a> CLIClient {
    const CERT_PEM: &'a str = "-----BEGIN CERTIFICATE-----
MIIBUjCB+aADAgECAgkAz1vuzG6opxQwCgYIKoZIzj0EAwIwITEfMB0GA1UEAwwW
cmNnZW4gc2VsZiBzaWduZWQgY2VydDAgFw03NTAxMDEwMDAwMDBaGA80MDk2MDEw
MTAwMDAwMFowITEfMB0GA1UEAwwWcmNnZW4gc2VsZiBzaWduZWQgY2VydDBZMBMG
ByqGSM49AgEGCCqGSM49AwEHA0IABJHCy1MLoykAXS8sD1jXBDfpNeVzZAGJJ8Fv
Tu/7OrYj4kEomKbl0qn4uYK/wmEgPwjDoCe+2vg8FJTDT28txGSjGDAWMBQGA1Ud
EQQNMAuCCWxvY2FsaG9zdDAKBggqhkjOPQQDAgNIADBFAiBUOY7QT2ZUocjbt35I
f9C3ificV0wk6hvrp6sY4UQUTgIhAJFLMmmBC3o9NOJNaWpdTuUKFIzQZFl61gFu
MWcWnCgP
-----END CERTIFICATE-----";

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn run(&self) -> crossterm::Result<()> {
        // todo: resize window automatically

        let (endpoint, mut uni_streams) = Self::create_quic_client().await;

        while let Some(Ok(recv)) = uni_streams.next().await {
            // Because it is a unidirectional stream, we can only receive not send back.
            let serialized_message = match recv.read_to_end(1_000_000).await {
                Ok(serialized_message) => serialized_message,
                Err(err) => {
                    panic!("{err}")
                }
            };

            Self::print_serialized_message(&serialized_message).await?;
        }

        // Give the server has a chance to clean up
        endpoint.wait_idle().await;
        Ok(())
    }

    async fn create_quic_client() -> (Endpoint, IncomingUniStreams) {
        let server_addr = "127.0.0.1:5000".parse().unwrap();

        let cert_der = match rustls_pemfile::read_one(&mut Self::CERT_PEM.as_bytes())
            .unwrap()
            .unwrap()
        {
            rustls_pemfile::Item::X509Certificate(cert_der) => cert_der,
            _ => {
                unreachable!()
            }
        };

        let mut cert_root_store = rustls::RootCertStore::empty();
        cert_root_store.add(&rustls::Certificate(cert_der)).unwrap();

        let client_cfg = quinn::ClientConfig::with_root_certificates(cert_root_store);
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap()).unwrap();
        endpoint.set_default_client_config(client_cfg);

        let quinn::NewConnection {
            connection,
            uni_streams,
            ..
        } = endpoint
            .connect(server_addr, "localhost")
            .unwrap()
            .await
            .unwrap(); // todo: deal with timeout?
        println!("[client] connected: addr={}", connection.remote_address());
        (endpoint, uni_streams)
    }

    async fn print_serialized_message(serialized_message: &Vec<u8>) -> crossterm::Result<()> {
        let mut stdout = stdout();

        let num_newlines: u16;
        {
            let message_reader = capnp::serialize::read_message(
                &mut serialized_message.as_slice(),
                ::capnp::message::ReaderOptions::default(),
            )
            .unwrap();
            let computer_info = message_reader
                .get_root::<pchmd_capnp::computer_info::Reader>()
                .unwrap();

            stdout
                .queue(crossterm::style::Print(format!(
                    "Name: {}\n",
                    computer_info.get_name().unwrap()
                )))
                .unwrap();
            let (upper, lower) = (
                computer_info.get_uuid_upper(),
                computer_info.get_uuid_lower(),
            );
            stdout
                .queue(crossterm::style::Print(format!(
                    "UUID: {}\n",
                    uuid::Uuid::from_u64_pair(upper, lower).hyphenated()
                )))
                .unwrap();

            stdout
                .queue(crossterm::style::Print(format!(
                    "Operating System: {}\n",
                    computer_info.get_operating_system().unwrap()
                )))
                .unwrap();

            let version = computer_info.get_server_version().unwrap();
            stdout
                .queue(crossterm::style::Print(format!(
                    "Server Version: {}.{}.{}\n",
                    version.get_major(),
                    version.get_minor(),
                    version.get_patch()
                )))
                .unwrap();

            let sensor_data_table =
                Self::populate_sensor_data_table(computer_info.get_sensors().unwrap());
            let table_string = tabled::Table::new(sensor_data_table)
                .with(tabled::Style::rounded())
                .to_string();
            num_newlines = (table_string.chars().filter(|&c| c == '\n').count() + 4) as u16;
            stdout.queue(crossterm::style::Print(table_string)).unwrap();
            stdout.flush().unwrap();
        }

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Required since: https://github.com/crossterm-rs/crossterm/issues/673
        stdout
            .queue(crossterm::cursor::MoveToPreviousLine(num_newlines as u16))?
            .queue(crossterm::terminal::Clear(
                crossterm::terminal::ClearType::FromCursorDown,
            ))?;
        Ok(())
    }

    fn populate_sensor_data_table(
        sensors: capnp::struct_list::Reader<pchmd_capnp::sensor_data::Owned>,
    ) -> Vec<SensorDataCLITableEntry> {
        let mut sensor_data_table = Vec::new();

        for sensor in sensors.iter() {
            sensor_data_table.push(SensorDataCLITableEntry {
                sensor_name: sensor.get_sensor_name().unwrap().to_string(),
                data_source_name: sensor.get_data_source_name().unwrap().to_string(),
                current_value: match sensor.get_current().unwrap().which().unwrap() {
                    pchmd_capnp::sensor_value::WhichReader::FloatValue(value) => {
                        SensorValue::Float(value)
                    }
                    pchmd_capnp::sensor_value::WhichReader::BoolValue(value) => {
                        SensorValue::Bool(value)
                    }
                    pchmd_capnp::sensor_value::WhichReader::StringValue(value) => {
                        SensorValue::Text(value.unwrap().to_string())
                    }
                },
                average_value: match sensor.get_average().unwrap().which().unwrap() {
                    pchmd_capnp::sensor_value::WhichReader::FloatValue(value) => {
                        SensorValue::Float(value)
                    }
                    pchmd_capnp::sensor_value::WhichReader::BoolValue(value) => {
                        SensorValue::Bool(value)
                    }
                    pchmd_capnp::sensor_value::WhichReader::StringValue(value) => {
                        SensorValue::Text(value.unwrap().to_string())
                    }
                },
                minimum_value: match sensor.get_minimum().unwrap().which().unwrap() {
                    pchmd_capnp::sensor_value::WhichReader::FloatValue(value) => {
                        SensorValue::Float(value)
                    }
                    pchmd_capnp::sensor_value::WhichReader::BoolValue(value) => {
                        SensorValue::Bool(value)
                    }
                    pchmd_capnp::sensor_value::WhichReader::StringValue(value) => {
                        SensorValue::Text(value.unwrap().to_string())
                    }
                },
                maximum_value: match sensor.get_maximum().unwrap().which().unwrap() {
                    pchmd_capnp::sensor_value::WhichReader::FloatValue(value) => {
                        SensorValue::Float(value)
                    }
                    pchmd_capnp::sensor_value::WhichReader::BoolValue(value) => {
                        SensorValue::Bool(value)
                    }
                    pchmd_capnp::sensor_value::WhichReader::StringValue(value) => {
                        SensorValue::Text(value.unwrap().to_string())
                    }
                },
                // measurement_unit: match sensor.get_measurement_unit().unwrap() {
                //
                // },
                is_stale: sensor.get_is_stale(),
            });
        }
        sensor_data_table
    }
}

impl Default for CLIClient {
    fn default() -> Self {
        stdout().execute(crossterm::cursor::Hide).unwrap();
        Self {}
    }
}

impl Drop for CLIClient {
    fn drop(&mut self) {
        stdout().execute(crossterm::cursor::Show).unwrap();
    }
}

#[derive(Tabled)]
struct SensorDataCLITableEntry {
    sensor_name: String,
    data_source_name: String,

    current_value: SensorValue,
    average_value: SensorValue,
    minimum_value: SensorValue,
    maximum_value: SensorValue,

    // measurement_unit: Option<MeasurementUnit>,
    is_stale: bool,
}
