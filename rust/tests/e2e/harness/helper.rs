//! Helper functions for E2E tests

use std::time::Duration;
use tokio::time::timeout;

/// Wait for a condition with a timeout
#[allow(dead_code)]
pub async fn wait_for<F, Fut>(
    condition: F,
    timeout_duration: Duration,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    timeout(timeout_duration, async {
        loop {
            if condition().await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await?;

    Ok(())
}

/// Assert that a string contains a substring
#[macro_export]
macro_rules! assert_contains {
    ($haystack:expr, $needle:expr) => {
        assert!(
            $haystack.contains($needle),
            "Expected '{}' to contain '{}'",
            $haystack,
            $needle
        )
    };
}

/// Assert that a vector is not empty
#[macro_export]
macro_rules! assert_not_empty {
    ($vec:expr) => {
        assert!(!$vec.is_empty(), "Expected non-empty collection")
    };
}
