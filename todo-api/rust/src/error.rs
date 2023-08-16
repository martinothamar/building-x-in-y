use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct AppError {
    pub message: &'static str,
}

impl AppError {
    pub(crate) fn new(message: &'static str) -> Json<AppError> {
        Json(AppError { message })
    }
}
