use std::str::FromStr;

use anyhow::{ensure, Error, Result};
use uuid::Uuid;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub(crate) struct TodoId {
    value: Uuid,
}

impl From<TodoId> for Uuid {
    fn from(value: TodoId) -> Self {
        value.value
    }
}

impl TryFrom<Uuid> for TodoId {
    type Error = Error;

    fn try_from(value: Uuid) -> Result<Self, Self::Error> {
        ensure!(!value.is_nil(), "Invalid UUID");
        let version = value.get_version();
        ensure!(
            version == Some(uuid::Version::SortRand),
            "Invalid UUID - not v7: {:?}",
            version
        );
        Ok(Self { value })
    }
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

impl From<Ordering> for i64 {
    fn from(value: Ordering) -> Self {
        value.value
    }
}

impl Ordering {
    pub(crate) fn new(value: i64) -> Result<Self> {
        ensure!(value >= 0, "Invalid ordering - negative: {}", value);
        Ok(Ordering { value })
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

impl Todo {
    pub(crate) fn new(ordering: i64, title: String, description: String, done: Option<bool>) -> Result<Self> {
        ensure!(
            !title.is_empty() && title.len() < 2048,
            "Invalid title length: {}",
            title.len()
        );
        ensure!(
            !description.is_empty() && description.len() < 2048,
            "Invalid description length: {}",
            description.len()
        );
        Ok(Self {
            id: TodoId::new(),
            ordering: Ordering::new(ordering)?,
            title,
            description,
            done: done.unwrap_or(false),
        })
    }
}
