//! Pre-built quantum algorithms exposed by the server's `/algorithms` API.
//!
//! Obtain a handle with [`crate::Client::algorithms`] and call the per-algorithm
//! methods. Each returns a typed result deserialized from the server's
//! `{ algorithm, parameters, result }` envelope.

use crate::client::Client;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Envelope wrapping every algorithm response; only `result` is surfaced.
#[derive(Deserialize)]
struct Envelope<T> {
    result: T,
}

/// A weighted Pauli term of a VQE Hamiltonian.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PauliTerm {
    /// Scalar weight of the term.
    pub coefficient: f64,
    /// Pauli operators (e.g. `["Z", "Z"]`).
    pub paulis: Vec<String>,
    /// Qubits each Pauli acts on, aligned with `paulis`.
    pub qubits: Vec<usize>,
}

/// An example QAOA graph (a MaxCut instance).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QaoaGraph {
    /// Number of vertices / qubits.
    pub n: usize,
    /// Undirected edges as `(u, v)` pairs.
    pub edges: Vec<(usize, usize)>,
}

/// Metadata about an available algorithm (from `list`).
#[derive(Clone, Debug, Deserialize)]
pub struct AlgorithmInfo {
    /// Display name.
    pub name: String,
    /// One-line description.
    pub description: String,
    /// Category, e.g. `"search"`, `"cryptography"`.
    pub category: String,
}

#[derive(Deserialize)]
struct AlgorithmList {
    algorithms: Vec<AlgorithmInfo>,
}

#[derive(Deserialize)]
struct Examples<T> {
    examples: HashMap<String, T>,
}

/// Result of a Quantum Fourier Transform run.
#[derive(Clone, Debug, Deserialize)]
pub struct QftResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// Number of qubits.
    pub qubits: usize,
    /// Number of gates in the constructed circuit.
    #[serde(rename = "gateCount")]
    pub gate_count: usize,
    /// Circuit depth.
    pub depth: usize,
    /// Number of non-zero amplitudes in the resulting state.
    #[serde(rename = "stateSize")]
    pub state_size: usize,
}

/// Result of a Grover's search run.
#[derive(Clone, Debug, Deserialize)]
pub struct GroverResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// Probability of measuring the marked item.
    #[serde(rename = "successProbability")]
    pub success_probability: f64,
    /// Optimal number of Grover iterations for this problem size.
    #[serde(rename = "optimalIterations")]
    pub optimal_iterations: u32,
}

/// Result of a Shor's factoring run.
#[derive(Clone, Debug, Deserialize)]
pub struct ShorResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// The factors found.
    pub factors: Vec<i64>,
    /// The period discovered (`-1` if not applicable).
    pub period: i64,
    /// Number of attempts taken.
    pub attempts: u32,
}

/// Teleported single-qubit probabilities.
#[derive(Clone, Debug, Deserialize)]
pub struct TeleportedProbabilities {
    /// Probability of measuring `|0>` on the receiver.
    pub prob0: f64,
    /// Probability of measuring `|1>` on the receiver.
    pub prob1: f64,
}

/// Result of a quantum teleportation run.
#[derive(Clone, Debug, Deserialize)]
pub struct TeleportResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// Receiver-side measurement probabilities.
    #[serde(rename = "teleportedProbabilities")]
    pub teleported_probabilities: TeleportedProbabilities,
    /// State fidelity between sent and received states.
    pub fidelity: f64,
    /// Whether teleportation was verified within tolerance.
    pub verified: bool,
}

/// Result of a VQE run.
#[derive(Clone, Debug, Deserialize)]
pub struct VqeResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// Lowest energy found (ground-state estimate).
    #[serde(rename = "optimalEnergy")]
    pub optimal_energy: f64,
    /// Iterations performed.
    pub iterations: u32,
    /// Whether the optimizer converged.
    pub converged: bool,
}

/// Result of a QAOA run.
#[derive(Clone, Debug, Deserialize)]
pub struct QaoaResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// Maximum objective expectation reached.
    #[serde(rename = "maxExpectation")]
    pub max_expectation: f64,
    /// Best cut value found.
    #[serde(rename = "bestCutValue")]
    pub best_cut_value: f64,
    /// Optimal gamma angles.
    #[serde(rename = "optimalGamma")]
    pub optimal_gamma: Vec<f64>,
    /// Optimal beta angles.
    #[serde(rename = "optimalBeta")]
    pub optimal_beta: Vec<f64>,
}

/// Handle for the algorithms API, borrowed from a [`Client`].
pub struct Algorithms<'a> {
    pub(crate) client: &'a Client,
}

impl Algorithms<'_> {
    /// List the algorithms the server can run.
    pub async fn list(&self) -> Result<Vec<AlgorithmInfo>> {
        let resp: AlgorithmList = self.client.get("/algorithms").await?;
        Ok(resp.algorithms)
    }

    /// Run the Quantum Fourier Transform on `n` qubits.
    pub async fn qft(&self, n: usize) -> Result<QftResult> {
        self.run("/algorithms/qft", &serde_json::json!({ "n": n }))
            .await
    }

    /// Run Grover's search over `n` qubits for `marked_item`. Pass
    /// `iterations = None` to use the optimal iteration count.
    pub async fn grover(
        &self,
        n: usize,
        marked_item: usize,
        iterations: Option<u32>,
    ) -> Result<GroverResult> {
        let mut body = serde_json::json!({ "n": n, "markedItem": marked_item });
        if let Some(it) = iterations {
            body["iterations"] = serde_json::json!(it);
        }
        self.run("/algorithms/grover", &body).await
    }

    /// Run Shor's algorithm to factor `number`.
    pub async fn shor(&self, number: u64) -> Result<ShorResult> {
        self.run("/algorithms/shor", &serde_json::json!({ "N": number }))
            .await
    }

    /// Run quantum teleportation of the state `alpha|0> + beta|1>`.
    pub async fn teleport(&self, alpha: f64, beta: f64) -> Result<TeleportResult> {
        self.run(
            "/algorithms/teleport",
            &serde_json::json!({ "alpha": alpha, "beta": beta }),
        )
        .await
    }

    /// Run VQE for `n` qubits against `hamiltonian`. Pass `max_iterations = None`
    /// for the server default.
    pub async fn vqe(
        &self,
        n: usize,
        hamiltonian: &[PauliTerm],
        max_iterations: Option<u32>,
    ) -> Result<VqeResult> {
        let mut body = serde_json::json!({ "n": n, "hamiltonian": hamiltonian });
        if let Some(m) = max_iterations {
            body["maxIterations"] = serde_json::json!(m);
        }
        self.run("/algorithms/vqe", &body).await
    }

    /// Run QAOA for MaxCut on `n` vertices with the given `edges`. `p` is the
    /// number of layers (`None` uses the server default).
    pub async fn qaoa(
        &self,
        n: usize,
        edges: &[(usize, usize)],
        p: Option<u32>,
    ) -> Result<QaoaResult> {
        let mut body = serde_json::json!({ "n": n, "edges": edges });
        if let Some(p) = p {
            body["p"] = serde_json::json!(p);
        }
        self.run("/algorithms/qaoa", &body).await
    }

    /// Fetch the built-in example Hamiltonians for VQE (name → terms).
    pub async fn vqe_examples(&self) -> Result<HashMap<String, Vec<PauliTerm>>> {
        let resp: Examples<Vec<PauliTerm>> = self.client.get("/algorithms/vqe/examples").await?;
        Ok(resp.examples)
    }

    /// Fetch the built-in example graphs for QAOA (name → graph).
    pub async fn qaoa_examples(&self) -> Result<HashMap<String, QaoaGraph>> {
        let resp: Examples<QaoaGraph> = self.client.get("/algorithms/qaoa/examples").await?;
        Ok(resp.examples)
    }

    async fn run<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T> {
        let env: Envelope<T> = self.client.post(path, body).await?;
        Ok(env.result)
    }
}
