//! Error Handler
//!
//! Error handling, retry, and timeout management.

use crate::error::Error;
use std::time::Duration;

/// Retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum retries
    pub max_retries: u32,
    /// Initial delay
    pub initial_delay: Duration,
    /// Maximum delay
    pub max_delay: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Get delay for a given attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay =
            self.initial_delay.as_millis() as f32 * self.backoff_multiplier.powi(attempt as i32);
        Duration::from_millis(delay.min(self.max_delay.as_millis() as f32) as u64)
    }
}

/// Retry result
#[derive(Debug)]
pub enum RetryResult<T> {
    /// Success
    Success(T),
    /// Retry needed
    Retry(Error),
    /// Failed after all retries
    Failed(Error),
}

/// Error handler
pub struct ErrorHandler {
    /// Retry policy
    pub retry_policy: RetryPolicy,
}

impl ErrorHandler {
    /// Create a new error handler
    pub fn new() -> Self {
        Self {
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Determine if an error should trigger a retry
    pub fn should_retry(&self, error: &Error, attempt: u32) -> bool {
        if attempt >= self.retry_policy.max_retries {
            return false;
        }

        matches!(
            error,
            Error::ConnectionFailed(_) | Error::ConnectionTimeout(_) | Error::Network(_)
        )
    }
}

impl Default for ErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::new(3);

        assert_eq!(policy.max_retries, 3);
        assert!(policy.delay_for_attempt(0) < policy.delay_for_attempt(1));
    }

    #[test]
    fn test_error_handler() {
        let handler = ErrorHandler::new();

        let retry_error = Error::ConnectionFailed("test".to_string());
        assert!(handler.should_retry(&retry_error, 0));
        assert!(!handler.should_retry(&retry_error, 3));
    }
}
