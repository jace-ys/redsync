use crate::errors::RedsyncError;
use crate::redsync::Lock;

use std::time::Duration;

/// `Instance` represents an entity with locking and unlocking capabilities.
pub trait Instance {
    fn acquire(&self, lock: &Lock) -> Result<(), RedsyncError>;
    fn extend(&self, lock: &Lock) -> Result<(), RedsyncError>;
    fn release(&self, lock: &Lock) -> Result<(), RedsyncError>;
}

const LOCK_SCRIPT: &str = "\
return redis.call(\"set\", KEYS[1], ARGV[1], \"nx\", \"px\", ARGV[2])";

const UNLOCK_SCRIPT: &str = "\
if redis.call(\"get\", KEYS[1]) == ARGV[1] then
    return redis.call(\"del\", KEYS[1])
else
    return 0
end";

const EXTEND_SCRIPT: &str = "\
if redis.call(\"get\", KEYS[1]) == ARGV[1] then
    return redis.call(\"pexpire\", KEYS[1], ARGV[2])
else
    return 0
end";

/// `RedisInstance` is the implementation of the Instance trait for a Redis server.
pub struct RedisInstance {
    client: redis::Client,
}

impl RedisInstance {
    pub fn new<T: redis::IntoConnectionInfo>(params: T) -> Result<Self, RedsyncError> {
        let client = redis::Client::open(params).map_err(RedsyncError::RedisError)?;
        Ok(Self { client })
    }

    fn timeout(&self, ttl: &Duration) -> Duration {
        Duration::from_millis((ttl.as_millis() as f64 * 0.01) as u64)
    }
}

impl Instance for RedisInstance {
    fn acquire(&self, lock: &Lock) -> Result<(), RedsyncError> {
        let mut conn = self
            .client
            .get_connection_with_timeout(self.timeout(&lock.ttl))
            .map_err(RedsyncError::RedisError)?;

        let result = redis::Script::new(LOCK_SCRIPT)
            .key(&lock.resource)
            .arg(&lock.value)
            .arg(lock.ttl.as_millis() as u64)
            .invoke(&mut conn);

        match result {
            Ok(redis::Value::Okay) => Ok(()),
            Ok(redis::Value::Nil) => Err(RedsyncError::ResourceLocked),
            Ok(v) => Err(RedsyncError::UnexpectedResponse(v)),
            Err(e) => Err(RedsyncError::RedisError(e)),
        }
    }

    fn extend(&self, lock: &Lock) -> Result<(), RedsyncError> {
        let mut conn = self
            .client
            .get_connection_with_timeout(self.timeout(&lock.ttl))
            .map_err(RedsyncError::RedisError)?;

        let result = redis::Script::new(EXTEND_SCRIPT)
            .key(&lock.resource)
            .arg(&lock.value)
            .arg(lock.ttl.as_millis() as u64)
            .invoke(&mut conn);

        match result {
            Ok(redis::Value::Int(1)) => Ok(()),
            Ok(redis::Value::Int(0)) => Err(RedsyncError::InvalidLease),
            Ok(v) => Err(RedsyncError::UnexpectedResponse(v)),
            Err(e) => Err(RedsyncError::RedisError(e)),
        }
    }

    fn release(&self, lock: &Lock) -> Result<(), RedsyncError> {
        let mut conn = self
            .client
            .get_connection_with_timeout(self.timeout(&lock.ttl))
            .map_err(RedsyncError::RedisError)?;

        let result = redis::Script::new(UNLOCK_SCRIPT)
            .key(&lock.resource)
            .arg(&lock.value)
            .invoke(&mut conn);

        match result {
            Ok(redis::Value::Int(1)) => Ok(()),
            Ok(redis::Value::Int(0)) => Err(RedsyncError::InvalidLease),
            Ok(v) => Err(RedsyncError::UnexpectedResponse(v)),
            Err(e) => Err(RedsyncError::RedisError(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::matches;
    use std::thread;
    use std::time::Instant;

    struct TestHelper {
        instance: RedisInstance,
        lock: Lock,
    }

    fn setup(resource: &str) -> TestHelper {
        // These tests require running a redis server on 127.0.0.1:6379
        let instance = RedisInstance::new("redis://127.0.0.1:6379").unwrap();
        let lock = Lock {
            resource: String::from(resource),
            value: String::from("1"),
            ttl: Duration::from_millis(500),
            expiry: Instant::now(),
        };

        TestHelper { instance, lock }
    }

    #[test]
    fn url_error() {
        let instance = RedisInstance::new("127.0.0.1:6379");
        assert!(matches!(instance, Err(RedsyncError::RedisError { .. })));
    }

    #[test]
    fn acquire() {
        let test = setup("acquire");

        let attempt = test.instance.acquire(&test.lock);
        assert!(attempt.is_ok());
    }

    #[test]
    fn acquire_locked_resource() -> Result<(), RedsyncError> {
        let test = setup("acquire_locked_resource");
        test.instance.acquire(&test.lock)?;

        let attempt = test.instance.acquire(&test.lock);
        assert!(matches!(attempt, Err(RedsyncError::ResourceLocked)));

        Ok(())
    }

    #[test]
    fn extend() -> Result<(), RedsyncError> {
        let mut test = setup("extend");
        test.instance.acquire(&test.lock)?;

        test.lock.ttl = Duration::from_millis(100);
        let attempt = test.instance.extend(&test.lock);
        assert!(attempt.is_ok());

        Ok(())
    }

    #[test]
    fn extend_invalid_lock() -> Result<(), RedsyncError> {
        let mut test = setup("extend_invalid_lock");
        test.instance.acquire(&test.lock)?;

        test.lock.value = String::from("2");
        let attempt = test.instance.extend(&test.lock);
        assert!(matches!(attempt, Err(RedsyncError::InvalidLease)));

        Ok(())
    }

    #[test]
    fn extend_expired_lock() -> Result<(), RedsyncError> {
        let test = setup("extend_expired_lock");
        test.instance.acquire(&test.lock)?;
        thread::sleep(Duration::from_secs(1));

        let attempt = test.instance.extend(&test.lock);
        assert!(matches!(attempt, Err(RedsyncError::InvalidLease)));

        Ok(())
    }

    #[test]
    fn release() -> Result<(), RedsyncError> {
        let test = setup("release");
        test.instance.acquire(&test.lock)?;

        let attempt = test.instance.release(&test.lock);
        assert!(attempt.is_ok());

        Ok(())
    }

    #[test]
    fn release_invalid_lock() -> Result<(), RedsyncError> {
        let mut test = setup("unlock_invalid_lock");
        test.instance.acquire(&test.lock)?;

        test.lock.value = String::from("2");
        let attempt = test.instance.release(&test.lock);
        assert!(matches!(attempt, Err(RedsyncError::InvalidLease)));

        Ok(())
    }

    #[test]
    fn release_expired_lock() -> Result<(), RedsyncError> {
        let test = setup("unlock_expired_lock");
        test.instance.acquire(&test.lock)?;
        thread::sleep(Duration::from_secs(1));

        let attempt = test.instance.release(&test.lock);
        assert!(matches!(attempt, Err(RedsyncError::InvalidLease)));

        Ok(())
    }
}
