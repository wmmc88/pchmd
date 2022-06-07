use pchmd::*;

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() {
    Server::new(
        vec![DataSource::Libsensors(LibsensorsDataSource::new())],
        vec![TransportInterface::QUIC(QUICTransportInterface::new())],
    )
    .run()
}
