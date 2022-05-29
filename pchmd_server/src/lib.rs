#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]

extern crate core;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use capnp::serialize_packed;
use lm_sensors::prelude::*;
use lm_sensors::value::Unit;
use lm_sensors::Value;
use rand::prelude::*;
use rand_pcg::Pcg64Mcg;
use rand_seeder::Seeder;

pub mod pchmd_capnp {
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
    pub fn new(data_sources: Vec<DataSource>, transport_interfaces: Vec<TransportInterface>) -> Self {
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
        for data_source in &self.data_sources {
            data_source.update_values(&mut self.sensor_data, &self.ewma_alpha_value);
        }
        let mut serialized_msg = self.serialize_to_capnproto();
        for interface in &self.transport_interfaces {
            interface.send_message(&serialized_msg);
        }
    }

    fn serialize_to_capnproto(&self) -> Vec<u8> {
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
                        SensorValue::Bool(value) => {
                            current.set_bool_value(value.round() as u8 != 0);
                        }
                        SensorValue::Text(value) => {
                            current.set_string_value(value.as_str());
                        }
                    }
                }
                let mut average = sensor_data.reborrow().init_average();
                {
                    match &sensor_data_value.average_value {
                        SensorValue::Float(value) => {
                            average.set_float_value(*value);
                        }
                        SensorValue::Bool(value) => {
                            average.set_bool_value(value.round() as u8 != 0);
                        }
                        SensorValue::Text(value) => {
                            average.set_string_value(value.as_str());
                        }
                    }
                }
                let mut minimum = sensor_data.reborrow().init_minimum();
                {
                    match &sensor_data_value.minimum_value {
                        SensorValue::Float(value) => {
                            minimum.set_float_value(*value);
                        }
                        SensorValue::Bool(value) => {
                            minimum.set_bool_value(value.round() as u8 != 0);
                        }
                        SensorValue::Text(value) => {
                            minimum.set_string_value(value.as_str());
                        }
                    }
                }
                let mut maximum = sensor_data.reborrow().init_maximum();
                {
                    match &sensor_data_value.maximum_value {
                        SensorValue::Float(value) => {
                            maximum.set_float_value(*value);
                        }
                        SensorValue::Bool(value) => {
                            maximum.set_bool_value(value.round() as u8 != 0);
                        }
                        SensorValue::Text(value) => {
                            maximum.set_string_value(value.as_str());
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
        serialize_packed::write_message(&mut buffer, &message).unwrap();
        buffer
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

    fn update_values(&self, sensor_data_map: &mut SensorDataMap, ewma_alpha_value: &f64) {
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
    Bool(f64), // f64 as type to be able to min/max/average subsequent values
    Text(String),
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

    fn update_values(&self, sensor_data_map: &mut SensorDataMap, EWMA_ALPHA_VALUE: &f64) {
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
                                                let average_value = *EWMA_ALPHA_VALUE
                                                    * current_value
                                                    + (1.0 - *EWMA_ALPHA_VALUE) * average_value;
                                                sensor_data.average_value =
                                                    SensorValue::Float(average_value);
                                            }
                                        }
                                        SensorValue::Bool(average_value) => {
                                            if let SensorValue::Bool(current_value) = sensor_value {
                                                let average_value = *EWMA_ALPHA_VALUE
                                                    * current_value
                                                    + (1.0 - *EWMA_ALPHA_VALUE) * average_value;
                                                sensor_data.average_value =
                                                    SensorValue::Float(average_value);
                                            }
                                        }
                                        SensorValue::Text(average_value) => {
                                            // TODO: should have a count and set average value to highest count
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
                                        SensorValue::Bool(minimum_value) => {
                                            if let SensorValue::Bool(current_value) = sensor_value {
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
                                    SensorValue::Bool(maximum_value) => {
                                        if let SensorValue::Bool(current_value) = sensor_value {
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
            | Value::BeepEnable(value) => Ok(SensorValue::Bool(f64::from(i8::from(*value)))),

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
    fn send_message(&self, serialized_message: &Vec<u8>) {
        match self {
            TransportInterface::QUIC(transport_interface) => {
                transport_interface.send_message(serialized_message);
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct QUICTransportInterface {}

impl QUICTransportInterface {
    fn send_message(&self, serialized_message: &Vec<u8>) {
        todo!();
    }
}

pub struct CLIClient {}

impl CLIClient {
    // fn smth(){
    // #[derive(Tabled)]
    // struct Language {
    //     name: &'static str,
    //     designed_by: &'static str,
    //     invented_year: usize,
    // }
    //
    // let mut stdout = stdout();
    // stdout.execute(cursor::Hide).unwrap();
    //
    // for year in 1950..2030 {
    // let languages = vec ! [
    // Language {
    // name: "C",
    // designed_by: "Dennis Ritchie",
    // invented_year: 1972,
    // },
    // Language {
    // name: "Rust",
    // designed_by: "Graydon Hoare",
    // invented_year: year,
    // },
    // Language {
    // name: "Go",
    // designed_by: "Rob Pike",
    // invented_year: 2009,
    // },
    // ];
    // let table_string = Table::new(languages).with(Style::rounded()).to_string();
    // let num_newlines = table_string.chars().filter( | & c| c == '\n').count();
    //
    // stdout.queue(cursor::SavePosition).unwrap();
    // stdout.write_all(table_string.as_bytes()).unwrap();
    // stdout.flush().unwrap();
    //
    // thread::sleep(time::Duration::from_millis(200));
    // stdout.queue(cursor::RestorePosition).unwrap();
    // stdout.queue(cursor::MoveUp(num_newlines as u16)).unwrap(); // Required since: https://github.com/crossterm-rs/crossterm/issues/673
    // stdout
    // .queue(terminal::Clear(terminal::ClearType::FromCursorDown))
    // .unwrap();
    // }
    //
    // stdout.execute(cursor::Show).unwrap();
    // println!("Done!");
}

    // fn print_serialized_message(serialized_message: &mut Vec<u8>) {
    //     let message_reader = serialize_packed::read_message(
    //         &mut serialized_message.as_slice(),
    //         ::capnp::message::ReaderOptions::default(),
    //     ).unwrap();
    //     let computer_info = message_reader.get_root::<pchmd_capnp::computer_info::Reader>().unwrap();
    //
    //     println!("Name: {}", computer_info.get_name().unwrap());
    //     let (upper, lower) = (
    //         computer_info.get_uuid_upper(),
    //         computer_info.get_uuid_lower(),
    //     );
    //     println!("UUID: {}", uuid::Uuid::from_u64_pair(upper, lower).hyphenated());
    //     println!(
    //         "Operating System: {}",
    //         computer_info.get_operating_system().unwrap()
    //     );
    //     let version = computer_info.get_server_version().unwrap();
    //     println!(
    //         "Server Version: {}.{}.{}",
    //         version.get_major(),
    //         version.get_minor(),
    //         version.get_patch()
    //     );
    //     for sensor in computer_info.get_sensors().unwrap().iter() {
    //         println!(
    //             "{} from {}",
    //             sensor.get_sensor_name().unwrap(),
    //             sensor.get_data_source_name().unwrap()
    //         );
    //         print!("current: ");
    //         match sensor.get_current().unwrap().which().unwrap() {
    //             pchmd_capnp::sensor_value::WhichReader::FloatValue(value) => {
    //                 print!("{value}");
    //             }
    //             pchmd_capnp::sensor_value::WhichReader::BoolValue(value) => {
    //                 print!("{value}");
    //             }
    //             pchmd_capnp::sensor_value::WhichReader::StringValue(value) => {
    //                 print!("{}", value.unwrap());
    //             }
    //         };
    //
    //         print!(", average: ");
    //         match sensor.get_average().unwrap().which().unwrap() {
    //             pchmd_capnp::sensor_value::WhichReader::FloatValue(value) => {
    //                 print!("{value}");
    //             }
    //             pchmd_capnp::sensor_value::WhichReader::BoolValue(value) => {
    //                 print!("{value}");
    //             }
    //             pchmd_capnp::sensor_value::WhichReader::StringValue(value) => {
    //                 print!("{}", value.unwrap());
    //             }
    //         };
    //
    //         print!(", minimum: ");
    //         match sensor.get_minimum().unwrap().which().unwrap() {
    //             pchmd_capnp::sensor_value::WhichReader::FloatValue(value) => {
    //                 print!("{value}");
    //             }
    //             pchmd_capnp::sensor_value::WhichReader::BoolValue(value) => {
    //                 print!("{value}");
    //             }
    //             pchmd_capnp::sensor_value::WhichReader::StringValue(value) => {
    //                 print!("{}", value.unwrap());
    //             }
    //         };
    //
    //         print!(", maximum: ");
    //         match sensor.get_maximum().unwrap().which().unwrap() {
    //             pchmd_capnp::sensor_value::WhichReader::FloatValue(value) => {
    //                 print!("{value}");
    //             }
    //             pchmd_capnp::sensor_value::WhichReader::BoolValue(value) => {
    //                 print!("{value}");
    //             }
    //             pchmd_capnp::sensor_value::WhichReader::StringValue(value) => {
    //                 print!("{}", value.unwrap());
    //             }
    //         };
    //     }
    // }
}
