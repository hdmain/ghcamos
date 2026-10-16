//! Tiny async retry helper for optional upstream APIs.

use std::future::Future;
use std::time::Duration;

/// Run `f` up to `attempts` times with linear backoff (300ms, 600ms, …).
pub async fn with_retry<T, E, F, Fut>(attempts: u32, label: &str, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let attempts = attempts.max(1);
    let mut last_err: Option<E> = None;

    for attempt in 1..=attempts {
        match f().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                eprintln!("{label}: attempt {attempt}/{attempts} failed: {err}");
                last_err = Some(err);
                if attempt < attempts {
                    let delay_ms = 300u64.saturating_mul(attempt as u64);
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    Err(last_err.expect("attempts >= 1"))
}
