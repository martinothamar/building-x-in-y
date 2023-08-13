use anyhow::Result;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    httpsrv::server::start()?;
    Ok(())
}
