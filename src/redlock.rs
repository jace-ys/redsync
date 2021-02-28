use crate::builder::RedlockBuilder;
use crate::errors::{MultiError, RedlockError};
use crate::instance::Instance;

use std::ops::{Add, Sub};
use std::thread;
use std::time::{Duration, Instant};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub struct Lock {
    pub resource: String,
    pub value: String,
    pub ttl: Duration,
    pub expiry: Instant,
}

pub struct Redlock<I: Instance> {
    pub(crate) cluster: Vec<I>,
    pub(crate) quorum: u32,
    pub(crate) retry_count: u32,
    pub(crate) retry_delay: Duration,
    pub(crate) retry_jitter: f64,
    pub(crate) clock_drift_factor: f64,
}

enum Call {
    Lock,
    Extend,
}

impl<I: Instance> Redlock<I> {
    pub fn new(cluster: Vec<I>) -> Self {
        RedlockBuilder::new(cluster).build()
    }

    pub fn lock(&self, resource: &str, ttl: Duration) -> Result<Lock, RedlockError> {
        let value = self.get_unique_lock_id();
        self.call(Call::Lock, resource, &value, ttl)
    }

    pub fn extend(&self, lock: &Lock, ttl: Duration) -> Result<Lock, RedlockError> {
        self.call(Call::Extend, &lock.resource, &lock.value, ttl)
    }

    fn call(
        &self,
        call: Call,
        resource: &str,
        value: &str,
        ttl: Duration,
    ) -> Result<Lock, RedlockError> {
        let drift = Duration::from_millis(
            (ttl.as_millis() as f64 * self.clock_drift_factor as f64) as u64 + 2,
        );

        let mut errors = MultiError::new();

        for attempt in 1..=self.retry_count {
            let mut votes = 0;
            let start = Instant::now();

            let lock = Lock {
                resource: String::from(resource),
                value: String::from(value),
                ttl,
                expiry: start + ttl - drift,
            };

            for instance in &self.cluster {
                let result = match call {
                    Call::Lock => instance.acquire(&lock),
                    Call::Extend => instance.extend(&lock),
                };

                match result {
                    Ok(()) => votes += 1,
                    Err(e) => errors.push(e),
                }
            }

            if votes >= self.quorum && lock.expiry > Instant::now() {
                return Ok(lock);
            }

            let _ = self.unlock(&lock);
            if attempt < self.retry_count {
                errors.reset();
                thread::sleep(self.get_retry_delay());
            }
        }

        Err(RedlockError::LockRetriesExceeded(errors))
    }

    pub fn unlock(&self, lock: &Lock) -> Result<(), RedlockError> {
        let mut n = 0;
        let mut errors = MultiError::new();

        for instance in &self.cluster {
            match instance.release(&lock) {
                Ok(()) => n += 1,
                Err(e) => errors.push(e),
            };
        }

        if n < self.quorum {
            return Err(RedlockError::UnlockFailed(errors));
        }

        Ok(())
    }

    fn get_unique_lock_id(&self) -> String {
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(20)
            .map(char::from)
            .collect()
    }

    fn get_retry_delay(&self) -> Duration {
        let jitter = thread_rng().gen_range(-1.0..1.0) * self.retry_jitter;
        if jitter > 0.0 {
            self.retry_delay.add(Duration::from_millis(jitter as u64))
        } else {
            self.retry_delay.sub(Duration::from_millis(-jitter as u64))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::matches;

    struct FakeInstance {
        acquire: i32,
        extend: i32,
        release: i32,
    }

    impl FakeInstance {
        pub fn new(acquire: i32, extend: i32, release: i32) -> Self {
            Self {
                acquire,
                extend,
                release,
            }
        }
    }

    impl Instance for FakeInstance {
        fn acquire(&self, _lock: &Lock) -> Result<(), RedlockError> {
            match self.acquire {
                1 => Ok(()),
                _ => Err(RedlockError::ResourceLocked),
            }
        }

        fn extend(&self, _lock: &Lock) -> Result<(), RedlockError> {
            match self.extend {
                1 => Ok(()),
                _ => Err(RedlockError::InvalidLease),
            }
        }

        fn release(&self, _lock: &Lock) -> Result<(), RedlockError> {
            match self.release {
                1 => Ok(()),
                _ => Err(RedlockError::InvalidLease),
            }
        }
    }

    #[test]
    fn lock() {
        let dlm = Redlock::new(vec![
            FakeInstance::new(1, 1, 1),
            FakeInstance::new(1, 1, 1),
            FakeInstance::new(0, 1, 1),
        ]);

        let attempt = dlm.lock("test", Duration::from_secs(1));
        assert!(attempt.is_ok());

        let lock = attempt.unwrap();
        assert_eq!(lock.resource, "test");
        assert!(lock.value.len() > 0);
        assert_eq!(lock.ttl, Duration::from_secs(1));
    }

    #[test]
    fn lock_error() {
        let dlm = Redlock::new(vec![
            FakeInstance::new(0, 1, 1),
            FakeInstance::new(0, 1, 1),
            FakeInstance::new(1, 1, 1),
        ]);

        let attempt = dlm.lock("test", Duration::from_secs(1));
        assert!(matches!(
            attempt,
            Err(RedlockError::LockRetriesExceeded { .. })
        ));
    }

    #[test]
    fn extend() -> Result<(), RedlockError> {
        let dlm = Redlock::new(vec![
            FakeInstance::new(1, 1, 1),
            FakeInstance::new(1, 1, 1),
            FakeInstance::new(1, 0, 1),
        ]);
        let lock = dlm.lock("test", Duration::from_secs(1))?;

        let attempt = dlm.extend(&lock, Duration::from_secs(2));
        assert!(attempt.is_ok());

        let lock = attempt.unwrap();
        assert_eq!(lock.resource, "test");
        assert!(lock.value.len() > 0);
        assert_eq!(lock.ttl, Duration::from_secs(2));

        Ok(())
    }

    #[test]
    fn extend_error() -> Result<(), RedlockError> {
        let dlm = Redlock::new(vec![
            FakeInstance::new(1, 0, 1),
            FakeInstance::new(1, 0, 1),
            FakeInstance::new(1, 1, 1),
        ]);
        let lock = dlm.lock("test", Duration::from_secs(1))?;

        let attempt = dlm.extend(&lock, Duration::from_secs(2));
        assert!(matches!(
            attempt,
            Err(RedlockError::LockRetriesExceeded { .. })
        ));

        Ok(())
    }

    #[test]
    fn unlock() -> Result<(), RedlockError> {
        let dlm = Redlock::new(vec![
            FakeInstance::new(1, 1, 1),
            FakeInstance::new(1, 1, 1),
            FakeInstance::new(1, 1, 0),
        ]);
        let lock = dlm.lock("test", Duration::from_secs(1))?;

        let attempt = dlm.unlock(&lock);
        assert!(attempt.is_ok());

        Ok(())
    }

    #[test]
    fn unlock_error() -> Result<(), RedlockError> {
        let dlm = Redlock::new(vec![
            FakeInstance::new(1, 1, 0),
            FakeInstance::new(1, 1, 0),
            FakeInstance::new(1, 1, 1),
        ]);
        let lock = dlm.lock("test", Duration::from_secs(1))?;

        let attempt = dlm.unlock(&lock);
        assert!(matches!(attempt, Err(RedlockError::UnlockFailed { .. })));

        Ok(())
    }

    #[test]
    fn get_unique_lock_id() {
        let cluster = vec![FakeInstance::new(1, 1, 1)];
        let dlm = Redlock::new(cluster);

        let value = dlm.get_unique_lock_id();
        assert_eq!(value.len(), 20);
        assert!(value.is_ascii());
    }

    #[test]
    fn get_retry_delay() {
        let cluster = vec![FakeInstance::new(1, 1, 1)];
        let dlm = Redlock::new(cluster);

        let retry_delay = dlm.get_retry_delay();
        let (min, max) = (Duration::from_millis(100), Duration::from_millis(300));
        assert!(
            min < retry_delay && retry_delay < max,
            "expected retry delay to be between {:?} and {:?}, but got {:?}",
            min,
            max,
            retry_delay,
        );
    }
}
