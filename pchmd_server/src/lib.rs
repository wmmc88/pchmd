pub mod pchmd_capnp {
    include!(concat!(env!("OUT_DIR"), "/pchmd_capnp.rs"));
}

pub trait DataSource<'a> {
    const NAME: &'a str;

    fn new() -> Self;
    fn init(&mut self);
    fn get_version(&self) -> Option<&str>;
}

#[derive(Default)]
pub struct LibsensorsDataSource {
    lm_sensors_handle: Option<Box<lm_sensors::LMSensors>>,
    version: Option<String>,
}

impl<'a> DataSource<'a> for LibsensorsDataSource {
    const NAME: &'a str = "libsensors (lm-sensors library)";

    fn new() -> Self {
        LibsensorsDataSource::default()
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
}
