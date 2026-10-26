#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Initializing Atelier daemon...");
}
