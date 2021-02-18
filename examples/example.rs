use redlock::RedlockBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = vec![
        "redis:/127.0.0.1:6379",
        "redis://127.0.0.1:6389",
        "redis://127.0.0.1:6399",
    ];

    let dlm = RedlockBuilder::new(cluster)
        .retry_count(5)
        .build()
        .map_err(|err| format!("Failed to initialize Redlock client: {}", err))?;

    dlm.lock();
    Ok(())
}
