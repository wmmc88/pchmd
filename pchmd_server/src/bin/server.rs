use pchmd_server::*;

fn main() {
    let mut data_source = LibsensorsDataSource::new();
    data_source.init();
    println!(
        "LibSensors Version: {}",
        data_source.get_version().unwrap_or("Unknown")
    );

    println!("SERVER FINISHED EXECUTING?");
}
