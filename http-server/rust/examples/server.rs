use std::error::Error;

use tracing_subscriber;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    httpsrv::start()?;
    Ok(())
}
