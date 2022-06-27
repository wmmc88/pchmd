use pchmd::*;

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main() {
    Server::new(
        vec![DataSource::Libsensors(LibsensorsDataSource::new().await)],
        vec![TransportInterface::QUIC(QUICTransportInterface::new())],
    )
    .run().await;
}
