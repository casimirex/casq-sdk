//! Execution backends: list the available targets (simulators, emulated/real
//! hardware) and run a circuit on a chosen one.
//!
//! Obtain a handle with [`crate::Client::backends`]. Selecting where a circuit
//! runs is just a backend id — the request is otherwise identical across
//! simulators and hardware.

use crate::advanced::NoiseChannelConfig;
use crate::circuit::Circuit;
use crate::client::Client;
use crate::error::Result;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// What a backend can run.
#[derive(Clone, Debug, Deserialize)]
pub struct BackendCapabilities {
    /// Maximum number of qubits.
    #[serde(rename = "maxQubits")]
    pub max_qubits: usize,
    /// Gate set the backend runs natively (others require transpilation).
    #[serde(rename = "nativeGates")]
    pub native_gates: Vec<String>,
    /// Whether the backend models noise.
    #[serde(rename = "supportsNoise")]
    pub supports_noise: bool,
    /// Qubit connectivity (`"all-to-all"` or `"linear"`).
    pub connectivity: String,
    /// False only for a real quantum processor.
    pub simulated: bool,
}

/// An execution backend and its current availability.
#[derive(Clone, Debug, Deserialize)]
pub struct Backend {
    /// Backend id, e.g. `"local-simulator"`, `"emulated-qpu"`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// `"simulator"`, `"hardware-emulator"`, or `"hardware"`.
    #[serde(rename = "type")]
    pub backend_type: String,
    /// One-line description.
    pub description: String,
    /// Whether the backend can currently accept work.
    pub available: bool,
    /// The backend's capabilities.
    pub capabilities: BackendCapabilities,
}

/// The normalized result of a backend run.
#[derive(Clone, Debug, Deserialize)]
pub struct BackendRunResult {
    /// The backend that ran the circuit.
    #[serde(rename = "backendId")]
    pub backend_id: String,
    /// Number of qubits.
    #[serde(rename = "numQubits")]
    pub num_qubits: usize,
    /// Shots sampled.
    pub shots: u32,
    /// Sampled measurement counts.
    pub counts: HashMap<String, u64>,
    /// Per-basis-state probabilities.
    pub probabilities: HashMap<String, f64>,
    /// Backend-specific metadata (timing, engine, purity, native fraction, ...).
    pub metadata: Value,
}

impl BackendRunResult {
    /// Execution time in milliseconds, if present.
    pub fn execution_time_ms(&self) -> Option<f64> {
        self.metadata.get("executionTimeMs").and_then(Value::as_f64)
    }

    /// Purity Tr(ρ²), for noise-capable backends.
    pub fn purity(&self) -> Option<f64> {
        self.metadata.get("purity").and_then(Value::as_f64)
    }

    /// Fraction of the circuit's operations already in the backend's native set.
    pub fn native_gate_fraction(&self) -> Option<f64> {
        self.metadata
            .get("nativeGateFraction")
            .and_then(Value::as_f64)
    }
}

/// Options for a backend run.
#[derive(Clone, Debug, Default)]
pub struct BackendRunOptions {
    /// Measurement shots.
    pub shots: Option<u32>,
    /// Seed for reproducible sampling.
    pub seed: Option<u32>,
    /// Noise channels, for backends that support them.
    pub noise: Vec<NoiseChannelConfig>,
}

#[derive(Deserialize)]
struct BackendList {
    backends: Vec<Backend>,
}

/// Handle for the backends API, borrowed from a [`Client`].
pub struct Backends<'a> {
    pub(crate) client: &'a Client,
}

impl Backends<'_> {
    /// List the available execution backends and their capabilities.
    pub async fn list(&self) -> Result<Vec<Backend>> {
        let resp: BackendList = self.client.get("/backends").await?;
        Ok(resp.backends)
    }

    /// Fetch a single backend by id.
    pub async fn get(&self, id: &str) -> Result<Backend> {
        self.client.get(&format!("/backends/{id}")).await
    }

    /// Run `circuit` on the backend with the given id.
    pub async fn run(
        &self,
        id: &str,
        circuit: &Circuit,
        options: BackendRunOptions,
    ) -> Result<BackendRunResult> {
        let mut body = serde_json::json!({
            "numQubits": circuit.num_qubits(),
            "operations": circuit.operations(),
        });
        if let Some(s) = options.shots {
            body["shots"] = serde_json::json!(s);
        }
        if let Some(s) = options.seed {
            body["seed"] = serde_json::json!(s);
        }
        if !options.noise.is_empty() {
            body["noise"] = serde_json::json!(options.noise);
        }
        self.client
            .post(&format!("/backends/{id}/run"), &body)
            .await
    }
}
