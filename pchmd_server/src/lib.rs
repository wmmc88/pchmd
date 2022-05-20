use std::mem::MaybeUninit;

pub mod pchmd_capnp {
    include!(concat!(env!("OUT_DIR"), "/pchmd_capnp.rs"));
}

// TODO: Add libsensors integration tests (ex. sanity tests)
mod libsensors {
    #![allow(non_camel_case_types)]
    #![allow(non_upper_case_globals)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    #![allow(clippy::upper_case_acronyms)]
    include!(concat!(env!("OUT_DIR"), "/libsensors-bindings.rs"));
}

pub trait DataSource {
    fn new() -> Self;
    fn init(&mut self);
}

#[derive(Default)]
pub struct LibsensorsDataSource<'a> {
    version: Option<&'a str>,
}

impl<'a> DataSource for LibsensorsDataSource<'a> {
    fn new() -> Self {
        LibsensorsDataSource::default()
    }

    fn init(&mut self) {
        // TODO: setup handlers for sensors_parse_error_wfn and sensors_fatal_error
        // TODO: farhenheit option
        // TODO: support libsensors config file

        unsafe {
            let errno = libsensors::sensors_init(std::ptr::null_mut());
            if errno != 0 {
                let error_string = libsensors::sensors_strerror(errno);
                let error_string = if error_string.is_null() {
                    "libsensors failed to translate errno to readable error"
                } else {
                    std::ffi::CStr::from_ptr(error_string).to_str().expect("Invalid C string encoding encountered while converting lib_sensors strerror output!")
                };
                panic!("libsensors errno {errno}: {error_string}");
                // TODO: return result type instead of panic
            }

            if !libsensors::libsensors_version.is_null() {
                self.version = Some(std::ffi::CStr::from_ptr(libsensors::libsensors_version).to_str().expect("Invalid C string encoding encountered while converting libsensors_version string!"));
            }

            // Printing Sensor Readings
            let mut chip_number = 0;
            let mut sensors_chip_name =
                libsensors::sensors_get_detected_chips(std::ptr::null_mut(), &mut chip_number);
            while !sensors_chip_name.is_null() {
                // TODO: break out into get chip name fn
                const MAX_NAME_LENGTH: usize = 200;
                let mut chip_name = MaybeUninit::<[std::os::raw::c_char; MAX_NAME_LENGTH]>::uninit();
                if libsensors::sensors_snprintf_chip_name(
                    chip_name.as_mut_ptr() as *mut std::os::raw::c_char,
                    MAX_NAME_LENGTH as u64,
                    sensors_chip_name,
                ) < 0
                {
                    panic!("Failed to extract chip name from libsensors::sensors_chip_name type!");
                } else {
                    let chip_name = std::ffi::CStr::from_ptr(
                        chip_name.assume_init().as_mut_ptr() as *mut std::os::raw::c_char
                    )
                        .to_str()
                        .expect("Invalid C string encoding encountered while converting chip name!");
                    println!("{chip_name}");
                }

                // TODO: break out into get adapter name fn
                let adapter_name = libsensors::sensors_get_adapter_name(&(*sensors_chip_name).bus);
                let adapter_name = if adapter_name.is_null() {
                    "Can't get adapter name"
                } else {
                    std::ffi::CStr::from_ptr(adapter_name).to_str().expect(
                        "Invalid C string encoding encountered while converting adapter name!",
                    )
                };
                println!("Adapter: {adapter_name}");

                // TODO: break out into print chip function
                let mut feature_number = 0;
                let mut sensors_feature =
                    libsensors::sensors_get_features(sensors_chip_name, &mut feature_number);
                while !sensors_feature.is_null() {
                    match (*sensors_feature).type_ {
                        libsensors::sensors_feature_type_SENSORS_FEATURE_IN => {
                            let label = libsensors::sensors_get_label(sensors_chip_name, sensors_feature);
                            if label.is_null() {
                                let name_str = std::ffi::CStr::from_ptr((*sensors_feature).name).to_str().expect("Invalid C string encoding encountered while converting feature name!");
                                panic!("ERROR: Can't get label of feature {}", name_str);
                            }
                            let label_str = std::ffi::CStr::from_ptr(label).to_str().expect("Invalid C string encoding encountered while converting label name!");
                            println!("label: {label_str}");
                            libc::free(label as *mut std::ffi::c_void);

                            let sensors_subfeature = libsensors::sensors_get_subfeature(sensors_chip_name, sensors_feature, libsensors::sensors_subfeature_type_SENSORS_SUBFEATURE_IN_INPUT);
                            if !sensors_subfeature.is_null() {
                                if let Ok(value) = get_sensor_value(sensors_chip_name, sensors_subfeature) {
                                    println!("{value}");
                                } else {
                                    println!("     N/A  ")
                                }
                            } else {
                                println!("     N/A  ");
                            }
                        }

                        libsensors::sensors_feature_type_SENSORS_FEATURE_FAN => {
                            // print_chip_fan(name, feature, label_size)
                        }

                        libsensors::sensors_feature_type_SENSORS_FEATURE_TEMP => {
                            // print_chip_temp(name, feature, label_size)
                        }

                        libsensors::sensors_feature_type_SENSORS_FEATURE_POWER => {
                            // print_chip_power(name, feature, label_size)
                        }
                        libsensors::sensors_feature_type_SENSORS_FEATURE_ENERGY => {
                            // print_chip_energy(name, feature, label_size)
                        }

                        libsensors::sensors_feature_type_SENSORS_FEATURE_CURR => {
                            // print_chip_curr(name, feature, label_size)
                        }
                        libsensors::sensors_feature_type_SENSORS_FEATURE_HUMIDITY => {
                            // print_chip_humidity(name, feature, label_size)
                        }
                        libsensors::sensors_feature_type_SENSORS_FEATURE_VID => {
                            // print_chip_vid(name, feature, label_size)
                        }

                        libsensors::sensors_feature_type_SENSORS_FEATURE_INTRUSION => {
                            // print_chip_intrusion(name, feature, label_size)
                        }

                        libsensors::sensors_feature_type_SENSORS_FEATURE_BEEP_ENABLE => {
                            // print_chip_beep_enable(name, feature, label_size)
                        }

                        invalid_value @ (libsensors::sensors_feature_type_SENSORS_FEATURE_MAX_MAIN | libsensors::sensors_feature_type_SENSORS_FEATURE_MAX_OTHER | libsensors::sensors_feature_type_SENSORS_FEATURE_UNKNOWN) => {
                            panic!("{invalid_value} is an invalid sensors_feature_type value!");
                        }

                        unknown_value @ _ => {
                            panic!("{unknown_value} is an unsupported sensors_feature_type value!");
                        }
                    }
                }

                chip_number += 1;
                sensors_chip_name =
                    libsensors::sensors_get_detected_chips(std::ptr::null_mut(), &mut chip_number);
            }
        }
    }
}

impl<'a> Drop for LibsensorsDataSource<'a> {
    fn drop(&mut self) {
        unsafe {
            libsensors::sensors_cleanup();
        }
    }
}

impl<'a> LibsensorsDataSource<'a> {
    pub fn get_version(&self) -> &Option<&str> {
        &self.version
    }
}

// TODO: put this in a better place!
fn get_sensor_value(sensors_chip_name: *const libsensors::sensors_chip_name, subfeature: *const libsensors::sensors_subfeature) -> Result<f64, String> {
    let mut val = MaybeUninit::<f64>::uninit();
    let errno = unsafe { libsensors::sensors_get_value(sensors_chip_name, (*subfeature).number, val.as_mut_ptr()) };
    if errno != 0 && errno != -(libsensors::SENSORS_ERR_ACCESS_R as i32) {
        let subfeature_name = unsafe { std::ffi::CStr::from_ptr((*subfeature).name) }.to_str().expect("Invalid C string encoding encountered while converting lib_sensors strerror output!");

        let error_string = unsafe {libsensors::sensors_strerror(errno)};
        let error_string = if error_string.is_null() {
            "Unknown Error (libsensors failed to translate errno to readable error)"
        } else {
            unsafe { std::ffi::CStr::from_ptr(error_string) }.to_str().expect("Invalid C string encoding encountered while converting lib_sensors strerror output!")
        };

        Err(format!("ERROR: Can't get value of subfeature {}: {}", subfeature_name, error_string))
    } else {
        Ok(unsafe { val.assume_init() })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
