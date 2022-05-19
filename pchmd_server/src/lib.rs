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
                let mut chip_name: MaybeUninit<[std::os::raw::c_char; MAX_NAME_LENGTH]> =
                    MaybeUninit::uninit();
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

                // TODO: parse chip data (reimplement print chip logic)
                todo!("parse chip data (reimplement print chip logic)");

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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
