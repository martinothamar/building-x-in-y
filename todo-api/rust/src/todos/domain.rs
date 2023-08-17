use std::str::FromStr;

use uuid::Uuid;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub(crate) struct TodoId {
    value: Uuid,
}

impl TodoId {
    pub(crate) fn new() -> Self {
        Self { value: Uuid::now_v7() }
    }
}

pub(crate) type TodoIdParseError = uuid::Error;

impl FromStr for TodoId {
    type Err = TodoIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(|v| TodoId { value: v })
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub(crate) struct Ordering {
    value: i64,
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum NewOrderingError {
    #[error("tried to construct todo ordering with negative number")]
    Negative,
}

impl Ordering {
    pub(crate) fn new(value: i64) -> Result<Self, NewOrderingError> {
        match value {
            n if n >= 0 => Ok(Ordering { value: n }),
            _ => Err(NewOrderingError::Negative),
        }
    }
}

#[derive(getset::Getters)]
pub(crate) struct Todo {
    pub(crate) id: TodoId,
    pub(crate) ordering: Ordering,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) done: bool,
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum NewTodoError {
    #[error("couldnt construct ordering")]
    NewOrderingError(#[from] NewOrderingError),
}

impl Todo {
    pub(crate) fn new(
        ordering: i64,
        title: String,
        description: String,
        done: Option<bool>,
    ) -> Result<Self, NewTodoError> {
        Ok(Self {
            id: TodoId::new(),
            ordering: Ordering::new(ordering)?,
            title,
            description,
            done: done.unwrap_or(false),
        })
    }
}
