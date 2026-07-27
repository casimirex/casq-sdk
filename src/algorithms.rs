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
    /// The multiplicative order r found by quantum phase estimation (`-1` when a
    /// lucky GCD produced the factors without order finding).
    pub period: i64,
    /// The base `a` whose order was estimated (absent on the trivial paths).
    #[serde(default)]
    pub base: Option<i64>,
    /// Number of attempts taken.
    pub attempts: u32,
}

/// Result of a Deutsch-Jozsa run.
#[derive(Clone, Debug, Deserialize)]
pub struct DeutschJozsaResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// The decided oracle class, `"constant"` or `"balanced"`.
    pub decision: String,
    /// The true oracle class.
    pub expected: String,
    /// Whether the decision matched the oracle.
    pub correct: bool,
    /// Probability the input register measured all-zeros.
    #[serde(rename = "allZeroProbability")]
    pub all_zero_probability: f64,
}

/// Result of a Bernstein-Vazirani run.
#[derive(Clone, Debug, Deserialize)]
pub struct BernsteinVaziraniResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// The hidden bit string.
    pub secret: u64,
    /// The recovered bit string.
    pub recovered: u64,
    /// The recovered string in binary.
    #[serde(rename = "recoveredBits")]
    pub recovered_bits: String,
    /// Whether recovery matched the secret.
    pub correct: bool,
    /// Probability of the recovered outcome.
    #[serde(rename = "successProbability")]
    pub success_probability: f64,
}

/// Result of a Simon's algorithm run.
#[derive(Clone, Debug, Deserialize)]
pub struct SimonResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// The hidden period.
    pub secret: u64,
    /// The recovered period.
    pub recovered: u64,
    /// The recovered period in binary.
    #[serde(rename = "recoveredBits")]
    pub recovered_bits: String,
    /// Whether recovery matched the period.
    pub correct: bool,
    /// Number of independent linear constraints obtained.
    #[serde(rename = "equationCount")]
    pub equation_count: usize,
}

/// Result of a quantum phase-estimation run.
#[derive(Clone, Debug, Deserialize)]
pub struct PhaseEstimationResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// The true eigenphase.
    #[serde(rename = "truePhase")]
    pub true_phase: f64,
    /// The estimated eigenphase.
    #[serde(rename = "estimatedPhase")]
    pub estimated_phase: f64,
    /// The measured counting-register integer.
    #[serde(rename = "measuredInteger")]
    pub measured_integer: u64,
    /// Number of counting qubits (bits of precision).
    #[serde(rename = "precisionBits")]
    pub precision_bits: usize,
    /// Absolute estimation error.
    pub error: f64,
    /// Probability of the most-likely outcome.
    #[serde(rename = "bestProbability")]
    pub best_probability: f64,
}

/// Result of an amplitude-amplification run.
#[derive(Clone, Debug, Deserialize)]
pub struct AmplitudeAmplificationResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// Good-state probability under the initial preparation.
    #[serde(rename = "initialProbability")]
    pub initial_probability: f64,
    /// Good-state probability after amplification.
    #[serde(rename = "finalProbability")]
    pub final_probability: f64,
    /// Theoretical amplitude sin²((2k+1)θ).
    #[serde(rename = "theoreticalProbability")]
    pub theoretical_probability: f64,
    /// Number of Q iterations applied.
    pub iterations: u32,
    /// Ratio of final to initial probability.
    pub amplification: f64,
}

/// A single node's occupation probability from a quantum walk.
#[derive(Clone, Debug, Deserialize)]
pub struct WalkPoint {
    /// The node (cycle position).
    pub position: usize,
    /// Probability of occupying that node.
    pub probability: f64,
}

/// Result of a discrete-time quantum-walk run.
#[derive(Clone, Debug, Deserialize)]
pub struct QuantumWalkResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// Number of nodes on the cycle (2ⁿ).
    pub nodes: usize,
    /// Mean signed displacement from the start.
    #[serde(rename = "meanDisplacement")]
    pub mean_displacement: f64,
    /// Standard deviation of the position (quantum, ballistic).
    #[serde(rename = "stdDev")]
    pub std_dev: f64,
    /// Standard deviation of a classical walk (√T, diffusive).
    #[serde(rename = "classicalStdDev")]
    pub classical_std_dev: f64,
    /// Ratio of quantum to classical spread.
    #[serde(rename = "spreadRatio")]
    pub spread_ratio: f64,
    /// Full position distribution.
    pub distribution: Vec<WalkPoint>,
}

/// A basis state's probability from a Hamiltonian-simulation run.
#[derive(Clone, Debug, Deserialize)]
pub struct StateProbability {
    /// The computational basis state (as an integer).
    pub state: u64,
    /// Its probability.
    pub probability: f64,
}

/// Result of a Trotterized Hamiltonian-simulation run.
#[derive(Clone, Debug, Deserialize)]
pub struct HamiltonianSimulationResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// Final-state probabilities over the computational basis.
    pub probabilities: Vec<StateProbability>,
}

/// A complex amplitude `re + i·im`.
#[derive(Clone, Debug, Deserialize)]
pub struct ComplexAmplitude {
    /// Real part.
    pub re: f64,
    /// Imaginary part.
    pub im: f64,
}

/// Result of an HHL linear-system solve.
#[derive(Clone, Debug, Deserialize)]
pub struct HhlResult {
    /// Execution time in milliseconds.
    #[serde(rename = "executionTime")]
    pub execution_time: f64,
    /// The exact classical solution A⁻¹b (normalised).
    #[serde(rename = "classicalSolution")]
    pub classical_solution: Vec<f64>,
    /// The quantum-prepared solution amplitudes.
    #[serde(rename = "quantumSolution")]
    pub quantum_solution: Vec<ComplexAmplitude>,
    /// Fidelity between the quantum and classical solutions.
    pub fidelity: f64,
    /// Post-selection success probability (ancilla measured |1⟩).
    #[serde(rename = "successProbability")]
    pub success_probability: f64,
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

    /// Run Deutsch-Jozsa on an `n`-qubit oracle. `oracle` is `"constant"` or
    /// `"balanced"`; `value` (0/1) selects the constant, `mask` the balanced
    /// parity (both optional — the server defaults them).
    pub async fn deutsch_jozsa(
        &self,
        n: usize,
        oracle: &str,
        value: Option<u8>,
        mask: Option<u64>,
    ) -> Result<DeutschJozsaResult> {
        let mut body = serde_json::json!({ "n": n, "oracle": oracle });
        if let Some(v) = value {
            body["value"] = serde_json::json!(v);
        }
        if let Some(m) = mask {
            body["mask"] = serde_json::json!(m);
        }
        self.run("/algorithms/deutsch-jozsa", &body).await
    }

    /// Run Bernstein-Vazirani to recover the hidden string `secret` over `n` bits.
    pub async fn bernstein_vazirani(
        &self,
        n: usize,
        secret: u64,
    ) -> Result<BernsteinVaziraniResult> {
        self.run(
            "/algorithms/bernstein-vazirani",
            &serde_json::json!({ "n": n, "secret": secret }),
        )
        .await
    }

    /// Run Simon's algorithm to recover the hidden period `secret` over `n` bits.
    pub async fn simon(&self, n: usize, secret: u64) -> Result<SimonResult> {
        self.run(
            "/algorithms/simon",
            &serde_json::json!({ "n": n, "secret": secret }),
        )
        .await
    }

    /// Run quantum phase estimation for eigenphase `phi` with `precision`
    /// counting qubits.
    pub async fn phase_estimation(
        &self,
        phi: f64,
        precision: usize,
    ) -> Result<PhaseEstimationResult> {
        self.run(
            "/algorithms/phase-estimation",
            &serde_json::json!({ "phi": phi, "precision": precision }),
        )
        .await
    }

    /// Run amplitude amplification. `angles` are per-qubit RY angles defining the
    /// state preparation A; `good_states` are the basis states to amplify;
    /// `iterations = None` uses the optimal count.
    pub async fn amplitude_amplification(
        &self,
        angles: &[f64],
        good_states: &[usize],
        iterations: Option<u32>,
    ) -> Result<AmplitudeAmplificationResult> {
        let mut body = serde_json::json!({ "angles": angles, "goodStates": good_states });
        if let Some(it) = iterations {
            body["iterations"] = serde_json::json!(it);
        }
        self.run("/algorithms/amplitude-amplification", &body).await
    }

    /// Run a discrete-time quantum walk on a cycle of `2ⁿ` nodes for `steps`
    /// steps. `start` defaults to the midpoint; `symmetric_coin` defaults to true.
    pub async fn quantum_walk(
        &self,
        n: usize,
        steps: u32,
        start: Option<usize>,
        symmetric_coin: Option<bool>,
    ) -> Result<QuantumWalkResult> {
        let mut body = serde_json::json!({ "n": n, "steps": steps });
        if let Some(s) = start {
            body["start"] = serde_json::json!(s);
        }
        if let Some(sc) = symmetric_coin {
            body["symmetricCoin"] = serde_json::json!(sc);
        }
        self.run("/algorithms/quantum-walk", &body).await
    }

    /// Run Trotterized time evolution e^{-iHt} of the Pauli-sum Hamiltonian
    /// `terms` on `n` qubits. `steps` (Trotter steps), `order` (1 or 2), and
    /// `initial_ones` (qubits to flip to |1⟩) are optional.
    pub async fn hamiltonian_simulation(
        &self,
        n: usize,
        terms: &[PauliTerm],
        time: f64,
        steps: Option<u32>,
        order: Option<u8>,
        initial_ones: Option<&[usize]>,
    ) -> Result<HamiltonianSimulationResult> {
        let mut body = serde_json::json!({ "n": n, "terms": terms, "time": time });
        if let Some(s) = steps {
            body["steps"] = serde_json::json!(s);
        }
        if let Some(o) = order {
            body["order"] = serde_json::json!(o);
        }
        if let Some(io) = initial_ones {
            body["initialOnes"] = serde_json::json!(io);
        }
        self.run("/algorithms/hamiltonian-simulation", &body).await
    }

    /// Run HHL to solve the canonical 2×2 system A x = b for the right-hand side
    /// `(b0, b1)`, preparing |x⟩ ∝ A⁻¹|b⟩.
    pub async fn hhl(&self, b0: f64, b1: f64) -> Result<HhlResult> {
        self.run(
            "/algorithms/hhl",
            &serde_json::json!({ "b0": b0, "b1": b1 }),
        )
        .await
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
