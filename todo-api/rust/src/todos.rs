use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::{
        atomic::{Ordering, AtomicUsize},
        Arc, Mutex,
    },
};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use axum_macros::debug_handler;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct CreateTodoDto {
    pub order: usize,
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TodoDto {
    pub id: u64,
    pub order: usize,
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Todo {
    pub order: usize,
    pub title: String,
    pub description: String,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Key(usize, usize);

struct Container {
    counter: AtomicUsize,
    db: Mutex<BTreeMap<Key, Todo>>,
}
type AppState = Arc<Container>;

pub(crate) const PATH: &str = "/todos";

pub(crate) fn router() -> Router {
    Router::new()
        .route("/", post(post_todo))
        .with_state(Arc::new(Container {
            counter: AtomicUsize::new(0),
            db: Mutex::new(BTreeMap::new()),
        }))
}

#[debug_handler]
async fn post_todo(State(state): State<AppState>, Json(todo): Json<CreateTodoDto>) -> Response {
    let count = state.counter.fetch_add(1, Ordering::Relaxed);
    let key = Key(todo.order, count);

    {
        let Some(mut db) = state.db.lock().ok() else {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Error").into_response();
        };

        match db.entry(key) {
            Entry::Vacant(v) => {
                let todo = Todo { order: todo.order, title: todo.title, description: todo.description };
                v.insert(todo);
            },
            Entry::Occupied(_) => return (StatusCode::BAD_REQUEST, "Todo already exists").into_response(),
        };
    }

    (StatusCode::CREATED, "").into_response()
}
