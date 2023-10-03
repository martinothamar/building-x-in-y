pub mod scalar;
pub mod vectorized;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EvaluationError {
    #[error("input length {0} does not match required input length {1}")]
    InvalidInputLength(usize, usize),
    #[error("input length {0} does not match required input length {1} at column {2}")]
    InvalidInputColumnLength(usize, usize, usize),
}
