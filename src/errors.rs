use std::fmt;
use std::ops::{Deref, DerefMut};

use thiserror::Error;

/// `RedsyncError` is an enum of all error kinds returned by the crate.
#[derive(Error, Debug, PartialEq)]
pub enum RedsyncError {
    #[error("{0}")]
    RedisError(#[from] redis::RedisError),
    #[error("unexpected response from Redis: {0:?}")]
    UnexpectedResponse(redis::Value),

    #[error("requested resource is current locked")]
    ResourceLocked,
    #[error("invalid or expired lease on lock")]
    InvalidLease,

    #[error("lock attempt failed: max retries exceeded: {0}")]
    LockRetriesExceeded(MultiError),
    #[error("extend attempt failed: max retries exceeded: {0}")]
    ExtendRetriesExceeded(MultiError),
    #[error("unlock attempt failed: {0}")]
    UnlockFailed(MultiError),
}

/// `MultiError` wraps `Vec<RedsyncError>`, typically aggregated over instances in a Redsync cluster.
#[derive(Debug, Default, PartialEq)]
pub struct MultiError(Vec<RedsyncError>);

impl MultiError {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn includes(&self, e: RedsyncError) -> bool {
        self.contains(&e)
    }

    pub(crate) fn reset(&mut self) {
        self.clear()
    }
}

impl Deref for MultiError {
    type Target = Vec<RedsyncError>;

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
