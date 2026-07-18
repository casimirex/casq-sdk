//! Error and result types for the casimirQ SDK.

/// The result type returned by every fallible SDK call.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur while talking to a casimirQ server.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The base URL passed to [`crate::Client::new`] could not be used.
    #[error("invalid base URL: {0}")]
    InvalidUrl(String),

    /// A request was made that requires authentication before [`crate::Client::login`]
    /// (or [`crate::Client::with_token`]) has provided a token.
    #[error("not authenticated: call `login` first or construct the client with a token")]
    NotAuthenticated,

    /// The server returned a non-success status. `message` is extracted from the
    /// casimirQ error envelope (`{ statusCode, error, message }`) when present.
    #[error("API error (HTTP {status}): {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Human-readable message from the server.
        message: String,
    },

    /// The underlying HTTP transport failed (DNS, TLS, connection, timeout, ...).
    #[error("HTTP transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// A response body could not be (de)serialized into the expected shape.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
