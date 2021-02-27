use crate::builder::RedlockBuilder;
use crate::errors::{MultiError, RedlockError};

use std::ops::Add;
use std::thread;
use std::time::{Duration, Instant};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

const LOCK_SCRIPT: &str = "\
return redis.call(\"set\", KEYS[1], ARGV[1], \"nx\", \"px\", ARGV[2])";

const UNLOCK_SCRIPT: &str = "\
if redis.call(\"get\", KEYS[1]) == ARGV[1] then
    return redis.call(\"del\", KEYS[1])
else
    return 0
end";

pub struct Lock {
    pub resource: String,
    pub value: String,
    pub ttl: Duration,
    pub expiry: Instant,
}

pub struct Redlock {
    pub(crate) clients: Vec<redis::Client>,
    pub(crate) quorum: u32,
    pub(crate) retry_count: u32,
    pub(crate) retry_delay: Duration,
    pub(crate) retry_jitter: u32,
    pub(crate) clock_drift_factor: f64,
    pub(crate) connection_timeout_factor: f64,
}

impl Redlock {
    pub fn new<T: redis::IntoConnectionInfo>(addrs: Vec<T>) -> Result<Self, RedlockError> {
        RedlockBuilder::new(addrs).build()
    }

    pub fn lock(&self, resource: &str, ttl: Duration) -> Result<Lock, RedlockError> {
        let value = self.get_unique_lock_id();
        let drift = Duration::from_millis(
            (ttl.as_millis() as f64 * self.clock_drift_factor as f64) as u64 + 2,
        );

        let mut errors = MultiError::new();

        for attempt in 1..=self.retry_count {
            let mut votes = 0;
            let start = Instant::now();

            let lock = Lock {
                resource: resource.to_owned(),
                value: value.to_owned(),
                ttl,
                expiry: start + ttl - drift,
            };

            for client in &self.clients {
                match self.lock_instance(client, &lock) {
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

    fn lock_instance(&self, client: &redis::Client, lock: &Lock) -> Result<(), RedlockError> {
        let timeout = self.get_connection_timeout(&lock.ttl);
        let mut conn = client
            .get_connection_with_timeout(timeout)
            .map_err(RedlockError::RedisError)?;

        let result = redis::Script::new(LOCK_SCRIPT)
            .key(&lock.resource)
            .arg(&lock.value)
            .arg(lock.ttl.as_millis() as u64)
            .invoke(&mut conn);

        match result {
            Ok(redis::Value::Okay) => Ok(()),
            Ok(redis::Value::Nil) => Err(RedlockError::ResourceLocked),
            Ok(v) => Err(RedlockError::InvalidResponse(v)),
            Err(e) => Err(RedlockError::RedisError(e)),
        }
    }

    pub fn unlock(&self, lock: &Lock) -> Result<(), RedlockError> {
        let mut n = 0;
        let mut errors = MultiError::new();

        for client in &self.clients {
            match self.unlock_instance(client, &lock) {
                Ok(()) => n += 1,
                Err(e) => errors.push(e),
            };
        }

        if n < self.quorum {
            return Err(RedlockError::UnlockFailed(errors));
        }

        Ok(())
    }

    fn unlock_instance(&self, client: &redis::Client, lock: &Lock) -> Result<(), RedlockError> {
        let timeout = self.get_connection_timeout(&lock.ttl);
        let mut conn = client
            .get_connection_with_timeout(timeout)
            .map_err(RedlockError::RedisError)?;

        let result = redis::Script::new(UNLOCK_SCRIPT)
            .key(&lock.resource)
            .arg(&lock.value)
            .invoke(&mut conn);

        match result {
            Ok(redis::Value::Int(1)) => Ok(()),
            Ok(redis::Value::Int(0)) => Err(RedlockError::InvalidLease),
            Ok(v) => Err(RedlockError::InvalidResponse(v)),
            Err(e) => Err(RedlockError::RedisError(e)),
        }
    }

    fn get_unique_lock_id(&self) -> String {
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(20)
            .map(char::from)
            .collect()
    }

    fn get_connection_timeout(&self, ttl: &Duration) -> Duration {
        Duration::from_millis(
            (ttl.as_millis() as f64 * self.connection_timeout_factor as f64) as u64,
        )
    }

    fn get_retry_delay(&self) -> Duration {
        let jitter = thread_rng().gen_range(0..self.retry_jitter);
        self.retry_delay.add(Duration::from_millis(jitter as u64))
    }
}
