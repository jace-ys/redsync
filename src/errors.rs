use std::fmt;
use std::ops::{Deref, DerefMut};

use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum RedlockError {
    #[error("{0}")]
    RedisError(#[from] redis::RedisError),
    #[error("invalid response from Redis server: {0:?}")]
    InvalidResponse(redis::Value),

    #[error("requested resource is current locked")]
    ResourceLocked,
    #[error("invalid or expired lease on lock")]
    InvalidLease,

    #[error("lock attempt failed: max retries exceeded: {0}")]
    LockRetriesExceeded(MultiError),
    #[error("unlock attempt failed: {0}")]
    UnlockFailed(MultiError),
}

#[derive(Debug, Default, PartialEq)]
pub struct MultiError(Vec<RedlockError>);

impl MultiError {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn reset(&mut self) {
        self.clear()
    }

    pub fn includes(&self, e: RedlockError) -> bool {
        self.contains(&e)
    }
}

impl Deref for MultiError {
    type Target = Vec<RedlockError>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MultiError {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for MultiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} errors occurred:", self.len())?;
        for error in self.iter() {
            write!(f, "\n\t * {}", error)?;
        }

        Ok(())
    }
}
