#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main() -> crossterm::Result<()> {
    pchmd_server::CLIClient::new().run().await

}
