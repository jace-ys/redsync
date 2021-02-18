use crate::errors::RedlockError;
use crate::redlock::Redlock;

use std::time::Duration;

pub struct RedlockBuilder<T: redis::IntoConnectionInfo> {
    addrs: Vec<T>,
    retry_count: u32,
    retry_delay: Duration,
    retry_jitter: u32,
}

impl<T: redis::IntoConnectionInfo> RedlockBuilder<T> {
    pub fn new(addrs: Vec<T>) -> Self {
        Self {
            addrs,
            retry_count: 3,
            retry_delay: Duration::from_millis(200),
            retry_jitter: 50,
        }
    }

    pub fn retry_count(mut self, retry_count: u32) -> Self {
        self.retry_count = retry_count;
        self
    }

    pub fn retry_delay(mut self, retry_delay: Duration) -> Self {
        self.retry_delay = retry_delay;
        self
    }

    pub fn retry_jitter(mut self, retry_jitter: u32) -> Self {
        self.retry_jitter = retry_jitter;
        self
    }

    pub fn build(self) -> Result<Redlock, RedlockError> {
        let mut clients: Vec<redis::Client> = Vec::with_capacity(self.addrs.len());
        for addr in self.addrs {
            clients.push(redis::Client::open(addr).map_err(RedlockError::RedisError)?);
        }

        let quorum = (clients.len() as u32) / 2 + 1;

        Ok(Redlock {
            clients,
            quorum,
            retry_count: self.retry_count,
            retry_delay: self.retry_delay,
            retry_jitter: self.retry_jitter,
            clock_drift_factor: 0.01,
        })
    }
}
