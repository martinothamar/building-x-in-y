use axum::Json;
use serde::Serialize;

pub(crate) enum StaticOrOwnedString {
    Static(&'static str),
    Owned(String),
}

impl Serialize for StaticOrOwnedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            StaticOrOwnedString::Static(s) => serializer.serialize_str(s),
            StaticOrOwnedString::Owned(s) => serializer.serialize_str(s),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct AppError {
    pub message: StaticOrOwnedString,
}

impl From<&'static str> for StaticOrOwnedString {
    fn from(val: &'static str) -> Self {
        StaticOrOwnedString::Static(val)
    }
}

impl From<String> for StaticOrOwnedString {
    fn from(val: String) -> Self {
        StaticOrOwnedString::Owned(val)
    }
}

impl AppError {
    pub(crate) fn new(message: StaticOrOwnedString) -> Json<AppError> {
        Json(AppError { message })
    }
}
