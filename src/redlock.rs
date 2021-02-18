use crate::builder::RedlockBuilder;
use crate::errors::RedlockError;

use std::time::Duration;

pub struct Redlock {
    pub(crate) clients: Vec<redis::Client>,
    pub(crate) quorum: u32,
    pub(crate) retry_count: u32,
    pub(crate) retry_delay: Duration,
    pub(crate) retry_jitter: u32,
    pub(crate) clock_drift_factor: f32,
}

impl Redlock {
    pub fn new<T: redis::IntoConnectionInfo>(addrs: Vec<T>) -> Result<Self, RedlockError> {
        RedlockBuilder::new(addrs).build()
    }

    pub fn lock(&self) {
        println!("{:?}", self.clients);
    }
}

pub struct Lock {}
