use crate::instance::Instance;
use crate::redlock::Redlock;

use std::time::Duration;

pub struct RedlockBuilder<I: Instance> {
    cluster: Vec<I>,
    retry_count: u32,
    retry_delay: Duration,
}

impl<I: Instance> RedlockBuilder<I> {
    pub fn new(cluster: Vec<I>) -> Self {
        Self {
            cluster,
            retry_count: 3,
            retry_delay: Duration::from_millis(200),
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

    pub fn build(self) -> Redlock<I> {
        let quorum = (self.cluster.len() as u32) / 2 + 1;
        let retry_jitter = self.retry_delay.as_millis() as f64 * 0.5;

        Redlock {
            cluster: self.cluster,
            quorum,
            retry_count: self.retry_count,
            retry_delay: self.retry_delay,
            retry_jitter,
            clock_drift_factor: 0.01,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::RedlockError;
    use crate::instance::RedisInstance;

    #[test]
    fn default() -> Result<(), RedlockError> {
        let cluster = vec![RedisInstance::new("redis://127.0.0.1:6379")?];
        let redlock = RedlockBuilder::new(cluster).build();

        assert_eq!(redlock.cluster.len(), 1);
        assert_eq!(redlock.quorum, 1);
        assert_eq!(redlock.retry_count, 3);
        assert_eq!(redlock.retry_delay, Duration::from_millis(200));
        assert_eq!(redlock.retry_jitter, 100.0);
        assert_eq!(redlock.clock_drift_factor, 0.01);

        Ok(())
    }

    #[test]
    fn retry_count() -> Result<(), RedlockError> {
        let cluster = vec![RedisInstance::new("redis://127.0.0.1:6379")?];
        let redlock = RedlockBuilder::new(cluster).retry_count(5).build();

        assert_eq!(redlock.cluster.len(), 1);
        assert_eq!(redlock.quorum, 1);
        assert_eq!(redlock.retry_count, 5);
        assert_eq!(redlock.retry_delay, Duration::from_millis(200));
        assert_eq!(redlock.retry_jitter, 100.0);
        assert_eq!(redlock.clock_drift_factor, 0.01);

        Ok(())
    }

    #[test]
    fn retry_delay() -> Result<(), RedlockError> {
        let cluster = vec![RedisInstance::new("redis://127.0.0.1:6379")?];
        let redlock = RedlockBuilder::new(cluster)
            .retry_delay(Duration::from_millis(100))
            .build();

        assert_eq!(redlock.cluster.len(), 1);
        assert_eq!(redlock.quorum, 1);
        assert_eq!(redlock.retry_count, 3);
        assert_eq!(redlock.retry_delay, Duration::from_millis(100));
        assert_eq!(redlock.retry_jitter, 50.0);
        assert_eq!(redlock.clock_drift_factor, 0.01);

        Ok(())
    }
}
