use anyhow::Result;
use sqlx::{pool::PoolConnection, Pool, Sqlite, SqlitePool};

use super::config;

#[derive(Clone)]
pub(crate) struct Db {
    pool: Pool<Sqlite>,
}

impl Db {
    pub(crate) async fn get_connection(&self) -> Result<PoolConnection<Sqlite>, sqlx::Error> {
        self.pool.acquire().await
    }
}

pub(crate) async fn init() -> Result<Db> {
    let conn_string = config::get().get_db_conn_string();
    let pool = SqlitePool::connect(conn_string).await?;

    sqlx::migrate!().run(&pool).await?;

    Ok(Db { pool })
}
