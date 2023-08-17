use crate::infra::db;
use anyhow::Result;
use sqlx::{query, types::Uuid};

use super::http::{CreateTodoDto, TodoDto};

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
