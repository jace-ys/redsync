use std::thread;
use std::time::Duration;

use redlock::{Redlock, RedlockError};

fn main() {
    if let Err(e) = run() {
        println!("{}", e);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = vec![
        "redis://127.0.0.1:6379",
        "redis://127.0.0.1:6389",
        "redis://127.0.0.1:6399",
    ];

    let dlm = Redlock::new(cluster)
        .map_err(|err| format!("Failed to initialize Redlock client: {}", err))?;

    let lock1 = dlm
        .lock("resource", Duration::from_secs(1))
        .map_err(|err| format!("Failed to acquire lock on resource: {}", err))?;
    println!("[t = 0] Acquired 1st lock for 1 second!");

    println!("[t = 0] Sleeping for 1 second!");
    thread::sleep(Duration::from_secs(1));

    let lock2 = dlm
        .lock("resource", Duration::from_secs(2))
        .map_err(|err| format!("Failed to acquire lock on resource: {}", err))?;
    println!("[t = 1] Acquired 2nd lock for 2 seconds!");

    println!("[t = 1] Sleeping for 1 second!");
    thread::sleep(Duration::from_secs(1));

    match dlm.unlock(&lock1) {
        Ok(()) => println!("[t = 2] Released 1st lock after 2 seconds!"),
        Err(RedlockError::UnlockFailed(err)) => {
            if err.includes(RedlockError::InvalidLease) {
                println!("[t = 2] Failed to release 1st lock. Lock has expired!")
            }
        }
        Err(_) => (),
    };

    dlm.extend(&lock2, Duration::from_secs(2))
        .map_err(|err| format!("Failed to extend lock on resource: {}", err))?;
    println!("[t = 2] Extended 2nd lock for 2 seconds!");

    println!("[t = 2] Sleeping for 1 second!");
    thread::sleep(Duration::from_secs(1));

    dlm.unlock(&lock2)
        .map_err(|err| format!("Failed to release lock on resource: {}", err))?;
    println!("[t = 3] Released 2nd lock after 1 second!");

    Ok(())
}
