//! The HTTP client that talks to a casimirQ server.

use crate::advanced::Advanced;
use crate::algorithms::Algorithms;
use crate::backends::Backends;
use crate::circuit::Circuit;
use crate::error::{Error, Result};
use crate::jobs::Jobs;
use crate::models::{AuthToken, CircuitList, CircuitRecord};
use crate::simulation::{RunOptions, SimulationResult};
use crate::transpile::TranspileResult;
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// An authenticated client for a casimirQ deployment.
///
/// Construct it with the API base URL (the part before the resource paths, e.g.
/// `http://localhost:8080/api/v1`), authenticate with [`Client::login`], then
/// build and run circuits.
///
/// # Example
/// ```no_run
/// use casq_sdk::{Circuit, Client, RunOptions};
///
/// # async fn run() -> casq_sdk::Result<()> {
/// let mut client = Client::new("http://localhost:8080/api/v1")?;
/// client.login("admin@example.com", "admin123").await?;
///
/// let mut circuit = Circuit::new(2);
/// circuit.h(0).cx(0, 1);
///
/// let result = client.run(&circuit, RunOptions::new().shots(1024)).await?;
/// println!("counts = {:?}", result.counts());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Client {
    base_url: String,
    http: reqwest::Client,
    token: Option<String>,
}

impl Client {
    /// Create a client for the given API base URL (no trailing slash required).
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let base_url = base_url.into();
        if base_url.trim().is_empty() {
            return Err(Error::InvalidUrl("base URL must not be empty".into()));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!("casq-sdk/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
            token: None,
        })
    }

    /// Create a client that is already authenticated with an existing token.
    pub fn with_token(base_url: impl Into<String>, token: impl Into<String>) -> Result<Self> {
        let mut client = Self::new(base_url)?;
        client.token = Some(token.into());
        Ok(client)
    }

    /// The API base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The current bearer token, if authenticated.
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Whether a token is set.
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    /// Set (or replace) the bearer token manually.
    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }

    // --- Authentication ---

    /// Log in with email + password. On success the returned token is stored on
    /// the client and used for subsequent requests.
    pub async fn login(&mut self, email: &str, password: &str) -> Result<AuthToken> {
        let body = serde_json::json!({ "email": email, "password": password });
        let token: AuthToken = self
            .send(Method::POST, "/auth/login", Some(&body), false)
            .await?;
        self.token = Some(token.access_token.clone());
        Ok(token)
    }

    /// Register a new account, then store the returned token.
    pub async fn signup(&mut self, email: &str, password: &str) -> Result<AuthToken> {
        let body = serde_json::json!({ "email": email, "password": password });
        let token: AuthToken = self
            .send(Method::POST, "/auth/signup", Some(&body), false)
            .await?;
        self.token = Some(token.access_token.clone());
        Ok(token)
    }

    // --- Circuits ---

    /// Persist a circuit under `name` and return the stored record.
    pub async fn create_circuit(&self, name: &str, circuit: &Circuit) -> Result<CircuitRecord> {
        let body = serde_json::json!({
            "name": name,
            "numQubits": circuit.num_qubits(),
            "operations": circuit.operations(),
        });
        self.post("/circuits", &body).await
    }

    /// List the current user's circuits (1-based `page`).
    pub async fn list_circuits(&self, page: usize, limit: usize) -> Result<CircuitList> {
        let path = format!("/circuits?page={page}&limit={limit}");
        self.get(&path).await
    }

    /// Fetch a single stored circuit by id.
    pub async fn get_circuit(&self, id: &str) -> Result<CircuitRecord> {
        self.get(&format!("/circuits/{id}")).await
    }

    /// Delete a stored circuit by id.
    pub async fn delete_circuit(&self, id: &str) -> Result<()> {
        self.send_discard(Method::DELETE, &format!("/circuits/{id}"), None)
            .await
    }

    // --- Simulation ---

    /// Simulate a circuit built locally (stateless; nothing is persisted).
    pub async fn run(&self, circuit: &Circuit, options: RunOptions) -> Result<SimulationResult> {
        let mut body = serde_json::json!({
            "numQubits": circuit.num_qubits(),
            "operations": circuit.operations(),
            "engine": options.engine,
        });
        if let Some(shots) = options.shots {
            body["shots"] = serde_json::json!(shots);
        }
        // The path id is ignored for inline runs; use a stable sentinel.
        self.post("/circuits/inline/simulate", &body).await
    }

    /// Decompose a circuit into the native gate basis (`rz`, `ry`, `cx`).
    pub async fn transpile(&self, circuit: &Circuit) -> Result<TranspileResult> {
        let body = serde_json::json!({
            "numQubits": circuit.num_qubits(),
            "operations": circuit.operations(),
        });
        self.post("/transpile", &body).await
    }

    /// Simulate a previously stored circuit by id.
    pub async fn run_stored(&self, id: &str, options: RunOptions) -> Result<SimulationResult> {
        let mut body = serde_json::json!({ "engine": options.engine });
        if let Some(shots) = options.shots {
            body["shots"] = serde_json::json!(shots);
        }
        self.post(&format!("/circuits/{id}/simulate"), &body).await
    }

    // --- Algorithms ---

    /// Access the pre-built quantum algorithms API.
    pub fn algorithms(&self) -> Algorithms<'_> {
        Algorithms { client: self }
    }

    /// Access the advanced-features API (error correction, noise, quantum ML).
    pub fn advanced(&self) -> Advanced<'_> {
        Advanced { client: self }
    }

    /// Access the execution backends API (simulators, emulated/real hardware).
    pub fn backends(&self) -> Backends<'_> {
        Backends { client: self }
    }

    /// Access the asynchronous job engine (submit, poll, cancel).
    pub fn jobs(&self) -> Jobs<'_> {
        Jobs { client: self }
    }

    // --- Internal request helpers (also used by the algorithms module) ---

    pub(crate) async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.send(Method::GET, path, None, true).await
    }

    pub(crate) async fn post<T: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        self.send(Method::POST, path, Some(body), true).await
    }

    pub(crate) async fn delete(&self, path: &str) -> Result<()> {
        self.send_discard(Method::DELETE, path, None).await
    }

    /// Send a request and deserialize the JSON body into `T`.
    async fn send<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
        auth: bool,
    ) -> Result<T> {
        let bytes = self.raw(method, path, body, auth).await?;
        serde_json::from_slice(&bytes).map_err(Error::Serde)
    }

    /// Send a request and discard the (possibly empty) body on success.
    async fn send_discard(&self, method: Method, path: &str, body: Option<&Value>) -> Result<()> {
        self.raw(method, path, body, true).await.map(|_| ())
    }

    /// Perform the request, returning the raw success body or an [`Error::Api`].
    async fn raw(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
        auth: bool,
    ) -> Result<Vec<u8>> {
        if auth && self.token.is_none() {
            return Err(Error::NotAuthenticated);
        }

        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let mut req = self.http.request(method, url);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        if let Some(body) = body {
            req = req.json(body);
        }

        let resp = req.send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;

        if status.is_success() {
            Ok(bytes.to_vec())
        } else {
            Err(Error::Api {
                status: status.as_u16(),
                message: extract_message(&bytes, status),
            })
        }
    }
}

/// Pull a human-readable message out of a casimirQ error envelope
/// (`{ statusCode, error, message }`, where `message` may be a string or list).
fn extract_message(bytes: &[u8], status: StatusCode) -> String {
    if let Ok(value) = serde_json::from_slice::<Value>(bytes) {
        match value.get("message") {
            Some(Value::String(s)) => return s.clone(),
            Some(Value::Array(items)) => {
                let joined = items
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                if !joined.is_empty() {
                    return joined;
                }
            }
            _ => {}
        }
        if let Some(Value::String(s)) = value.get("error") {
            return s.clone();
        }
    }
    status
        .canonical_reason()
        .unwrap_or("request failed")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trims_trailing_slash_and_rejects_empty() {
        let c = Client::new("http://localhost:8080/api/v1/").unwrap();
        assert_eq!(c.base_url(), "http://localhost:8080/api/v1");
        assert!(!c.is_authenticated());
        assert!(Client::new("   ").is_err());
    }

    #[test]
    fn with_token_is_authenticated() {
        let c = Client::with_token("http://x/api/v1", "abc").unwrap();
        assert!(c.is_authenticated());
        assert_eq!(c.token(), Some("abc"));
    }

    #[test]
    fn extract_message_handles_string_array_and_fallback() {
        let string = br#"{"statusCode":401,"error":"Unauthorized","message":"bad token"}"#;
        assert_eq!(
            extract_message(string, StatusCode::UNAUTHORIZED),
            "bad token"
        );

        let array = br#"{"message":["n must be >= 1","n must be <= 16"]}"#;
        assert_eq!(
            extract_message(array, StatusCode::BAD_REQUEST),
            "n must be >= 1; n must be <= 16"
        );

        let only_error = br#"{"error":"Not Found"}"#;
        assert_eq!(
            extract_message(only_error, StatusCode::NOT_FOUND),
            "Not Found"
        );

        // Non-JSON falls back to the HTTP reason phrase.
        assert_eq!(
            extract_message(b"<html>", StatusCode::INTERNAL_SERVER_ERROR),
            "Internal Server Error"
        );
    }
}
