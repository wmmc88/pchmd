use std::io::{stdout, Write};
use std::{error::Error, net::SocketAddr, sync::Arc};
use std::{thread, time};

use crossterm::{cursor, terminal, ExecutableCommand, QueueableCommand};
use futures_util::StreamExt;
use quinn::{ClientConfig, Endpoint, NewConnection, ReadToEndError};
use tabled::{Style, Table, Tabled};

use pchmd_server::pchmd_capnp;

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

const CERT_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIBUjCB+aADAgECAgkAz1vuzG6opxQwCgYIKoZIzj0EAwIwITEfMB0GA1UEAwwW
cmNnZW4gc2VsZiBzaWduZWQgY2VydDAgFw03NTAxMDEwMDAwMDBaGA80MDk2MDEw
MTAwMDAwMFowITEfMB0GA1UEAwwWcmNnZW4gc2VsZiBzaWduZWQgY2VydDBZMBMG
ByqGSM49AgEGCCqGSM49AwEHA0IABJHCy1MLoykAXS8sD1jXBDfpNeVzZAGJJ8Fv
Tu/7OrYj4kEomKbl0qn4uYK/wmEgPwjDoCe+2vg8FJTDT28txGSjGDAWMBQGA1Ud
EQQNMAuCCWxvY2FsaG9zdDAKBggqhkjOPQQDAgNIADBFAiBUOY7QT2ZUocjbt35I
f9C3ificV0wk6hvrp6sY4UQUTgIhAJFLMmmBC3o9NOJNaWpdTuUKFIzQZFl61gFu
MWcWnCgP
-----END CERTIFICATE-----";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server_addr = "127.0.0.1:5000".parse().unwrap();

    let cert_der = match rustls_pemfile::read_one(&mut CERT_PEM.as_bytes())
        .unwrap()
        .unwrap()
    {
        rustls_pemfile::Item::X509Certificate(cert_der) => cert_der,
        _ => {
            unreachable!()
        }
    };

    let server_certs = &[&cert_der];
    let mut cert_root_store = rustls::RootCertStore::empty();
    for cert in server_certs {
        cert_root_store.add(&rustls::Certificate(cert.to_vec()))?;
    }

    let client_cfg = ClientConfig::with_root_certificates(cert_root_store);
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())?;
    endpoint.set_default_client_config(client_cfg);

    let quinn::NewConnection {
        connection,
        mut uni_streams,
        ..
    } = endpoint
        .connect(server_addr, "localhost")
        .unwrap()
        .await
        .unwrap();
    println!("[client] connected: addr={}", connection.remote_address());

    while let Some(Ok(recv)) = uni_streams.next().await {
        // Because it is a unidirectional stream, we can only receive not send back.
        let serialized_message = match recv.read_to_end(1000000).await {
            Ok(serialized_message) => serialized_message,
            Err(err) => {
                panic!("{err}")
            }
        };

        let message_reader = capnp::serialize_packed::read_message(
            &mut serialized_message.as_slice(),
            ::capnp::message::ReaderOptions::default(),
        )
        .unwrap();
        let computer_info = message_reader
            .get_root::<pchmd_capnp::computer_info::Reader>()
            .unwrap();
        println!("{computer_info:?}");

        // println!("Name: {}", computer_info.get_name().unwrap());
        // let (upper, lower) = (
        //     computer_info.get_uuid_upper(),
        //     computer_info.get_uuid_lower(),
        // );
        // println!(
        //     "UUID: {}",
        //     uuid::Uuid::from_u64_pair(upper, lower).hyphenated()
        // );
        // println!(
        //     "Operating System: {}",
        //     computer_info.get_operating_system().unwrap()
        // );
        // let version = computer_info.get_server_version().unwrap();
        // println!(
        //     "Server Version: {}.{}.{}",
        //     version.get_major(),
        //     version.get_minor(),
        //     version.get_patch()
        // );
        // for sensor in computer_info.get_sensors().unwrap().iter() {
        //     println!(
        //         "{} from {}",
        //         sensor.get_sensor_name().unwrap(),
        //         sensor.get_data_source_name().unwrap()
        //     );
        //     print!("current: ");
        //     match sensor.get_current().unwrap().which().unwrap() {
        //         pchmd_capnp::sensor_value::WhichReader::FloatValue(value) => {
        //             print!("{value}");
        //         }
        //         pchmd_capnp::sensor_value::WhichReader::BoolValue(value) => {
        //             print!("{value}");
        //         }
        //         pchmd_capnp::sensor_value::WhichReader::StringValue(value) => {
        //             print!("{}", value.unwrap());
        //         }
        //     };
        //
        //     print!(", average: ");
        //     match sensor.get_average().unwrap().which().unwrap() {
        //         pchmd_capnp::sensor_value::WhichReader::FloatValue(value) => {
        //             print!("{value}");
        //         }
        //         pchmd_capnp::sensor_value::WhichReader::BoolValue(value) => {
        //             print!("{value}");
        //         }
        //         pchmd_capnp::sensor_value::WhichReader::StringValue(value) => {
        //             print!("{}", value.unwrap());
        //         }
        //     };
        //
        //     print!(", minimum: ");
        //     match sensor.get_minimum().unwrap().which().unwrap() {
        //         pchmd_capnp::sensor_value::WhichReader::FloatValue(value) => {
        //             print!("{value}");
        //         }
        //         pchmd_capnp::sensor_value::WhichReader::BoolValue(value) => {
        //             print!("{value}");
        //         }
        //         pchmd_capnp::sensor_value::WhichReader::StringValue(value) => {
        //             print!("{}", value.unwrap());
        //         }
        //     };
        //
        //     print!(", maximum: ");
        //     match sensor.get_maximum().unwrap().which().unwrap() {
        //         pchmd_capnp::sensor_value::WhichReader::FloatValue(value) => {
        //             print!("{value}");
        //         }
        //         pchmd_capnp::sensor_value::WhichReader::BoolValue(value) => {
        //             print!("{value}");
        //         }
        //         pchmd_capnp::sensor_value::WhichReader::StringValue(value) => {
        //             print!("{}", value.unwrap());
        //         }
        //     };
        // }
    }

    // Give the server has a chance to clean up
    endpoint.wait_idle().await;

    Ok(())
}
