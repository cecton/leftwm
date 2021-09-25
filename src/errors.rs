use thiserror::Error;

pub type Result<T> = std::result::Result<T, LeftError>;

#[derive(Debug, Error)]
pub enum LeftError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
