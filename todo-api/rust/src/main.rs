
use anyhow::Result;
use axum::Router;

mod health;
mod telemetry;
mod todos;

#[tokio::main]
async fn main() -> Result<()> {
    telemetry::init()?;

    let router = Router::new()
        .nest(health::PATH, health::router())
        .nest(todos::PATH, todos::router());

    axum::Server::bind(&"0.0.0.0:8080".parse()?).serve(router.into_make_service()).await?;

    telemetry::shutdown();
    Ok(())
}
