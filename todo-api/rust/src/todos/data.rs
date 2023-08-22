use crate::infra::db;
use anyhow::Result;
use sqlx::{query, types::Uuid};

use super::domain::{Ordering, Todo};

#[derive(Clone)]
pub(super) struct Repository {
    db: db::Db,
}

struct TodoEntity {
    pub(crate) id: Uuid,
    pub(crate) ordering: i64,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) done: bool,
}

impl From<Todo> for TodoEntity {
    fn from(value: Todo) -> Self {
        Self {
            id: value.id.into(),
            ordering: value.ordering.into(),
            title: value.title,
            description: value.description,
            done: value.done,
        }
    }
}

impl TryInto<Todo> for TodoEntity {
    type Error = anyhow::Error;

    fn try_into(self) -> std::result::Result<Todo, Self::Error> {
        Ok(Todo {
            id: self.id.try_into()?,
            ordering: Ordering::new(self.ordering)?,
            title: self.title,
            description: self.description,
            done: self.done,
        })
    }
}

impl Repository {
    pub(super) fn new(db: &db::Db) -> Self {
        Self { db: db.clone() }
    }

    pub(super) async fn create(&self, todo: Todo) -> Result<Uuid> {
        let mut conn = self.db.get_connection().await?;

        let todo: TodoEntity = todo.into();

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

    pub(super) async fn list(&self) -> Result<Vec<Todo>> {
        let mut conn = self.db.get_connection().await?;

        let todos = sqlx::query_as!(
            TodoEntity,
            r#"
                SELECT id AS "id: Uuid", ordering, title, description, done FROM todos
            "#
        )
        .fetch_all(&mut *conn)
        .await?;

        let todos = todos.into_iter().map(|t| t.try_into()).collect::<Result<Vec<_>, _>>()?;

        Ok(todos)
    }
}
