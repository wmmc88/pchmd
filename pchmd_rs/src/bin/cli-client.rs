#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main() -> crossterm::Result<()> {
    pchmd::CLIClient::new().run().await
}
