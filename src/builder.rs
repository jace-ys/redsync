use crate::instance::Instance;
use crate::redsync::Redsync;

use std::time::Duration;

pub struct RedsyncBuilder<I: Instance> {
    cluster: Vec<I>,
    retry_count: u32,
    retry_delay: Duration,
}

impl<I: Instance> RedsyncBuilder<I> {
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

    pub fn build(self) -> Redsync<I> {
        let quorum = (self.cluster.len() as u32) / 2 + 1;
        let retry_jitter = self.retry_delay.as_millis() as f64 * 0.5;

        Redsync {
            cluster: self.cluster,
            quorum,
            retry_count: self.retry_count,
            retry_delay: self.retry_delay,
            retry_jitter,
            drift_factor: 0.01,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::RedsyncError;
    use crate::instance::RedisInstance;

    #[test]
    fn default() -> Result<(), RedsyncError> {
        let cluster = vec![RedisInstance::new("redis://127.0.0.1:6379")?];
        let redsync = RedsyncBuilder::new(cluster).build();

        assert_eq!(redsync.cluster.len(), 1);
        assert_eq!(redsync.quorum, 1);
        assert_eq!(redsync.retry_count, 3);
        assert_eq!(redsync.retry_delay, Duration::from_millis(200));
        assert_eq!(redsync.retry_jitter, 100.0);
        assert_eq!(redsync.drift_factor, 0.01);

        Ok(())
    }

    #[test]
    fn retry_count() -> Result<(), RedsyncError> {
        let cluster = vec![RedisInstance::new("redis://127.0.0.1:6379")?];
        let redsync = RedsyncBuilder::new(cluster).retry_count(5).build();

        assert_eq!(redsync.cluster.len(), 1);
        assert_eq!(redsync.quorum, 1);
        assert_eq!(redsync.retry_count, 5);
        assert_eq!(redsync.retry_delay, Duration::from_millis(200));
        assert_eq!(redsync.retry_jitter, 100.0);
        assert_eq!(redsync.drift_factor, 0.01);

        Ok(())
    }

    #[test]
    fn retry_delay() -> Result<(), RedsyncError> {
        let cluster = vec![RedisInstance::new("redis://127.0.0.1:6379")?];
        let redsync = RedsyncBuilder::new(cluster)
            .retry_delay(Duration::from_millis(100))
            .build();

        assert_eq!(redsync.cluster.len(), 1);
        assert_eq!(redsync.quorum, 1);
        assert_eq!(redsync.retry_count, 3);
        assert_eq!(redsync.retry_delay, Duration::from_millis(100));
        assert_eq!(redsync.retry_jitter, 50.0);
        assert_eq!(redsync.drift_factor, 0.01);

        Ok(())
    }
}
