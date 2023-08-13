use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use axum::{extract::State, routing::get, Router, http::StatusCode, response::IntoResponse};

struct Container {
    counter: AtomicU64,
}
type AppState = Arc<Container>;

pub(crate) const PATH: &str = "/health";

pub(crate) fn router() -> Router {
    Router::new()
        .route(
            "/",
            get(get_endpoint),
        )
        .with_state(Arc::new(Container {
            counter: AtomicU64::new(0),
        }))
}

async fn get_endpoint(State(state): State<AppState>) -> impl IntoResponse {
    let count = state.counter.fetch_add(1, Ordering::Relaxed);
    (StatusCode::OK, format!("Healthy: {count}"))
}
