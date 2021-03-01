[![crates.io](https://img.shields.io/crates/v/redsync)](https://crates.io/crates/redsync)
[![docs.rs](https://docs.rs/redsync/badge.svg)](https://docs.rs/redsync)
[![ci](https://github.com/jace-ys/redsync/workflows/ci/badge.svg)](https://github.com/jace-ys/redsync/actions?query=workflow%3Aci)
[![release](https://github.com/jace-ys/redsync/workflows/release/badge.svg)](https://github.com/jace-ys/redsync/actions?query=workflow%3Arelease)

# Redsync

A Rust implementation of [Redlock](https://redis.io/topics/distlock) for distributed locks with Redis.

## Installation

Add the following line to your Cargo.toml file:

```toml
[dependencies]
redsync = "1.0.0"
```

## Documentation

See https://docs.rs/redsync.

# Quick Start

```rust
use std::error::Error;
use std::time::Duration;
use redsync::{RedisInstance, Redsync};

fn main() -> Result<(), Box<dyn Error>> {
  let dlm = Redsync::new(vec![
    RedisInstance::new("redis://127.0.0.1:6389")?,
    RedisInstance::new("redis://127.0.0.1:6399")?,
    RedisInstance::new("redis://127.0.0.1:6379")?,
  ]);

  let lock = dlm.lock("resource", Duration::from_secs(1))?;
  dlm.unlock(&lock)?;

  Ok(())
}
```

For more examples, see [examples](https://github.com/jace-ys/redsync/tree/master/examples).
