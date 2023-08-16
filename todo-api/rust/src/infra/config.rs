use std::{
    env,
    fmt::{Debug, Display},
    sync::OnceLock,
};

use tracing::info;

pub(crate) struct Config {
    inner: Box<ConfigInner>,
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

pub(crate) fn get() -> &'static Config {
    static CELL: OnceLock<Config> = OnceLock::new();

    CELL.get_or_init(|| {
        let config = Config {
            inner: Box::new(ConfigInner {
                address: env::var("ADDRESS").unwrap(),
                db_conn_string: env::var("DATABASE_URL").unwrap(),
                log_level: env::var("RUST_LOG").unwrap(),
            }),
        };

        info!(
            address = config.inner.address,
            log_level = config.inner.log_level,
            "initialized config"
        );

        config
    })
}

impl Config {
    pub(crate) fn get_address(&'static self) -> &'_ str {
        &self.inner.address
    }

    pub(crate) fn get_db_conn_string(&'static self) -> &'_ str {
        &self.inner.db_conn_string
    }
}

#[derive(Debug)]
struct ConfigInner {
    address: String,
    db_conn_string: String,
    log_level: String,
}
