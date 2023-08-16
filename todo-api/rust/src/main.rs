use anyhow::Result;
use axum::Router;
use tracing::info;
mod error;
mod health;
mod infra;
mod todos;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;
    infra::telemetry::init()?;
    let config = infra::config::get();

    let db = infra::db::init().await?;

    info!("initialized db");

    let router = Router::new()
        .nest(health::PATH, health::router())
        .nest(todos::PATH, todos::router(&db))
        .layer(infra::telemetry::tracing_middleware());

    info!("initialized router");

    axum::Server::bind(&config.get_address().parse()?)
        .serve(router.into_make_service())
        .with_graceful_shutdown(infra::os::shutdown_signal())
        .await?;

    infra::telemetry::shutdown();
    Ok(())
}
