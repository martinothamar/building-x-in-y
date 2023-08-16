use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::AppError, infra::db::Db};

use self::data::Repository;

#[derive(Serialize, Deserialize)]
pub(crate) struct CreateTodoDto {
    pub ordering: i64,
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TodoDto {
    pub id: Uuid,
    pub ordering: i64,
    pub title: String,
    pub description: String,
    pub done: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Todo {
    pub ordering: i64,
    pub title: String,
    pub description: String,
}

struct Container {
    repo: Repository,
}
impl Container {
    fn new(db: &Db) -> Arc<Self> {
        Arc::new(Container {
            repo: Repository::new(db),
        })
    }
}
type AppState = Arc<Container>;

pub(crate) const PATH: &str = "/todos";

pub(crate) fn router(db: &Db) -> Router {
    Router::new()
        .route("/", post(post_todo).get(list_todos))
        .with_state(Container::new(db))
}

// #[axum_macros::debug_handler]
async fn post_todo(State(state): State<AppState>, Json(todo): Json<CreateTodoDto>) -> Response {
    let repo = &state.repo;

    let Ok(_id) = repo.create(&todo).await else {
        return (StatusCode::INTERNAL_SERVER_ERROR, AppError::new("Db error")).into_response();
    };

    (StatusCode::CREATED, "").into_response()
}

async fn list_todos(State(state): State<AppState>) -> Response {
    let repo = &state.repo;

    let Ok(todos) = repo.list().await else {
        return (StatusCode::INTERNAL_SERVER_ERROR, AppError::new("Db error")).into_response();
    };

    (StatusCode::OK, Json(todos)).into_response()
}

pub(super) mod data {
    use crate::infra::db;
    use anyhow::Result;
    use sqlx::{query, types::Uuid};

    use super::{CreateTodoDto, TodoDto};

    #[derive(Clone)]
    pub(super) struct Repository {
        db: db::Db,
    }

    impl Repository {
        pub(super) fn new(db: &db::Db) -> Self {
            Self { db: db.clone() }
        }

        pub(super) async fn create(&self, todo: &CreateTodoDto) -> Result<Uuid> {
            let mut conn = self.db.get_connection().await?;

            let id = Uuid::now_v7();
            query!(
                r#"
                    INSERT INTO todos ( id, ordering, title, description, done )
                    VALUES ( ?1, ?2, ?3, ?4, ?5 )
                "#,
                id,
                todo.ordering,
                todo.title,
                todo.description,
                false
            )
            .execute(&mut *conn)
            .await?;

            Ok(id)
        }

        pub(super) async fn list(&self) -> Result<Vec<TodoDto>> {
            let mut conn = self.db.get_connection().await?;

            let todos = sqlx::query_as!(
                TodoDto,
                r#"
                    SELECT id AS "id: Uuid", ordering, title, description, done FROM todos
                "#
            )
            .fetch_all(&mut *conn)
            .await?;

            Ok(todos)
        }
    }
}
