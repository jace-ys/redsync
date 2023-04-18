//! # Installation
//!
//! Add the following line to your Cargo.toml file:
//!
//! ```toml
//! [dependencies]
//! redsync = "1.0.1"
//! ```
//!
//! # Quick Start
//!
//! ```rust
//! use std::error::Error;
//! use std::time::Duration;
//! use redsync::{RedisInstance, Redsync};
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!   let dlm = Redsync::new(vec![
//!     RedisInstance::new("redis://127.0.0.1:6389")?,
//!     RedisInstance::new("redis://127.0.0.1:6399")?,
//!     RedisInstance::new("redis://127.0.0.1:6379")?,
//!   ]);
//!
//!   let lock = dlm.lock("resource", Duration::from_secs(1))?;
//!   dlm.unlock(&lock)?;
//!
//!   Ok(())
//! }
//! ```
//!
//! For more examples, see [examples](https://github.com/jace-ys/redsync/tree/master/examples).
pub use crate::builder::RedsyncBuilder;
pub use crate::errors::{MultiError, RedsyncError};
pub use crate::instance::{Instance, RedisInstance};
pub use crate::redsync::{Lock, Redsync};

mod builder;
mod errors;
mod instance;
mod redsync;
