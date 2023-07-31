use anyhow::Result;

use tracing_subscriber;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    httpsrv::server2::start()?;
    Ok(())
}
