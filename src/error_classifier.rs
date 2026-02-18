use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Recoverable,    // Retry with normal backoff
    RateLimit,      // Retry with longer backoff
    Permanent,      // Don't retry
    Auth,           // Refresh token, then retry
}

/// Classify reqwest HTTP errors for retry logic
pub fn classify_http_error(error: &reqwest::Error) -> ErrorClass {
    // Check status code first
    if let Some(status) = error.status() {
        match status.as_u16() {
            429 => return ErrorClass::RateLimit,
            401 | 403 => return ErrorClass::Auth,
            400 | 404 | 422 => return ErrorClass::Permanent, // Client errors
            500..=599 => return ErrorClass::Recoverable,      // Server errors
            _ => {}
        }
    }

    // Check if it's a network/timeout error
    if error.is_timeout() || error.is_connect() {
        return ErrorClass::Recoverable;
    }

    // Default to recoverable for safety (give it a chance to retry)
    ErrorClass::Recoverable
}

/// Classify IO errors for retry logic
pub fn classify_io_error(error: &io::Error) -> ErrorClass {
    use io::ErrorKind::*;

    match error.kind() {
        // Network-related errors that might be transient
        TimedOut | ConnectionReset | ConnectionAborted | ConnectionRefused => {
            ErrorClass::Recoverable
        }
        // Permission errors might be auth-related
        PermissionDenied => ErrorClass::Auth,
        // These are permanent - don't retry
        NotFound | InvalidInput | InvalidData => ErrorClass::Permanent,
        // Everything else: retry once to be safe
        _ => ErrorClass::Recoverable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_io_timeout() {
        let err = io::Error::new(io::ErrorKind::TimedOut, "timeout");
        assert_eq!(classify_io_error(&err), ErrorClass::Recoverable);
    }

    #[test]
    fn test_classify_io_permission() {
        let err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        assert_eq!(classify_io_error(&err), ErrorClass::Auth);
    }

    #[test]
    fn test_classify_io_not_found() {
        let err = io::Error::new(io::ErrorKind::NotFound, "not found");
        assert_eq!(classify_io_error(&err), ErrorClass::Permanent);
    }

    #[test]
    fn test_classify_io_connection_reset() {
        let err = io::Error::new(io::ErrorKind::ConnectionReset, "reset");
        assert_eq!(classify_io_error(&err), ErrorClass::Recoverable);
    }
}
