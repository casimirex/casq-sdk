//! Shared data-transfer types (auth, users, stored circuits).

use crate::circuit::Operation;
use serde::Deserialize;

/// A user account as returned by the auth endpoints.
#[derive(Clone, Debug, Deserialize)]
pub struct User {
    /// Account email address.
    pub email: String,
}

/// The token envelope returned by `login` / `signup`.
#[derive(Clone, Debug, Deserialize)]
pub struct AuthToken {
    /// The JWT bearer token.
    #[serde(rename = "access_token")]
    pub access_token: String,
    /// Seconds until the token expires.
    #[serde(rename = "expires_in")]
    pub expires_in: u64,
    /// Token scheme, e.g. `"Bearer"`.
    #[serde(rename = "token_type")]
    pub token_type: String,
    /// The authenticated user.
    pub user: User,
}

/// A circuit as persisted by the server.
#[derive(Clone, Debug, Deserialize)]
pub struct CircuitRecord {
    /// Server-assigned id, e.g. `"circuit-..."`.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Number of qubits.
    #[serde(rename = "numQubits")]
    pub num_qubits: usize,
    /// The circuit's operations.
    #[serde(default)]
    pub operations: Vec<Operation>,
    /// Number of operations.
    #[serde(rename = "operationCount", default)]
    pub operation_count: usize,
    /// ISO-8601 creation timestamp.
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// A lightweight circuit summary as returned in list responses.
#[derive(Clone, Debug, Deserialize)]
pub struct CircuitSummary {
    /// Server-assigned id.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Number of qubits.
    #[serde(rename = "numQubits")]
    pub num_qubits: usize,
    /// Number of operations.
    #[serde(rename = "operationCount", default)]
    pub operation_count: usize,
    /// ISO-8601 creation timestamp.
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// Pagination metadata for a list response.
#[derive(Clone, Debug, Deserialize)]
pub struct Pagination {
    /// Current page (1-based).
    pub page: usize,
    /// Page size.
    pub limit: usize,
    /// Total number of items owned by the user.
    pub total: usize,
    /// Total number of pages.
    #[serde(rename = "totalPages")]
    pub total_pages: usize,
}

/// A paginated list of circuits.
#[derive(Clone, Debug, Deserialize)]
pub struct CircuitList {
    /// The circuits on this page.
    pub circuits: Vec<CircuitSummary>,
    /// Pagination metadata.
    pub pagination: Pagination,
}
