use thiserror::Error;

#[derive(Error, Debug)]
pub enum RedlockError {
    #[error("{0}")]
    RedisError(#[from] redis::RedisError),
}
