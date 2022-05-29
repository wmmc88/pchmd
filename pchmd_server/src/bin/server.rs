use pchmd_server::*;

fn main() {
    Server::new(
        vec![DataSource::Libsensors(LibsensorsDataSource::new())],
        vec![TransportInterface::QUIC(QUICTransportInterface {})],
    )
    .run()
}
