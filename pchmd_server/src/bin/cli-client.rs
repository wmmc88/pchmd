use std::error::Error;
use std::io::{stdout, Write};
use std::net::SocketAddr;
use std::{thread, time};

use crossterm::{cursor, terminal, ExecutableCommand, QueueableCommand};
use quinn::*;
use tabled::{Style, Table, Tabled};

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() {

}
use std::{error::Error, net::SocketAddr, sync::Arc};

use futures_util::StreamExt;
use quinn::{ClientConfig, Endpoint};

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

    let cert_der = match rustls_pemfile::read_one(&mut CERT_PEM.as_bytes()).unwrap().unwrap() {
        rustls_pemfile::Item::X509Certificate(cert_der) => {
            cert_der
        }
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

    // Waiting for a stream will complete with an error when the server closes the connection
    let _ = uni_streams.next().await;

    // Give the server has a chance to clean up
    endpoint.wait_idle().await;

    Ok(())
}
