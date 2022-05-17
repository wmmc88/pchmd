pub mod pchmd_capnp {
    include!(concat!(env!("OUT_DIR"), "/pchmd_capnp.rs"));
}

// TODO: Add libsensors integration tests (ex. sanity tests)
mod lib_sensors {
    #![allow(non_camel_case_types)]
    #![allow(non_upper_case_globals)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/libsensors-bindings.rs"));
}
