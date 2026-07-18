//! Simulation request options and result types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Which simulation engine to request. `Auto` lets the server choose (e.g. the
/// Clifford engine for stabilizer circuits, otherwise the statevector engine).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    /// Server-selected engine (the default).
    #[default]
    Auto,
    /// Dense statevector engine.
    Statevector,
    /// Clifford / stabilizer engine.
    Clifford,
    /// Matrix-product-state engine.
    Mps,
}

/// Options for a simulation run.
#[derive(Clone, Debug, Default, Serialize)]
pub struct RunOptions {
    /// Engine to use.
    pub engine: Engine,
    /// Number of measurement shots to sample. `None` uses the server default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shots: Option<u32>,
}

impl RunOptions {
    /// A fresh set of options (engine = `Auto`, server-default shots).
    pub fn new() -> Self {
        Self::default()
    }

    /// Select the engine.
    pub fn engine(mut self, engine: Engine) -> Self {
        self.engine = engine;
        self
    }

    /// Request a specific number of shots.
    pub fn shots(mut self, shots: u32) -> Self {
        self.shots = Some(shots);
        self
    }
}

/// One entry of the returned statevector.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Amplitude {
    /// Basis state as a bitstring, e.g. `"01"`.
    pub state: String,
    /// Real part of the amplitude.
    pub re: f64,
    /// Imaginary part of the amplitude.
    pub im: f64,
    /// Probability `|amplitude|²`.
    pub probability: f64,
}

/// The numerical outputs of a run.
#[derive(Clone, Debug, Deserialize)]
pub struct SimulationOutputs {
    /// Non-zero statevector amplitudes.
    pub statevector: Vec<Amplitude>,
    /// Per-basis-state probabilities.
    pub probabilities: HashMap<String, f64>,
    /// Sampled measurement counts (bitstring → occurrences).
    pub counts: HashMap<String, u64>,
}

/// Timing / resource metadata for a run.
#[derive(Clone, Debug, Deserialize)]
pub struct SimulationMetadata {
    /// Wall-clock execution time in milliseconds.
    #[serde(rename = "executionTimeMs")]
    pub execution_time_ms: f64,
    /// Approximate peak memory used, in bytes.
    #[serde(rename = "memoryUsageBytes")]
    pub memory_usage_bytes: u64,
}

/// The full result of a simulation run.
#[derive(Clone, Debug, Deserialize)]
pub struct SimulationResult {
    /// Identifier of the circuit that was run (or a synthetic id for inline runs).
    #[serde(rename = "circuitId")]
    pub circuit_id: String,
    /// Identifier of the recorded simulation job.
    #[serde(rename = "jobId")]
    pub job_id: String,
    /// Job status, typically `"completed"`.
    pub status: String,
    /// Number of qubits simulated.
    #[serde(rename = "numQubits")]
    pub num_qubits: usize,
    /// The engine the run was executed with.
    #[serde(rename = "requestedEngine")]
    pub requested_engine: String,
    /// Number of shots sampled.
    pub shots: u32,
    /// Numerical outputs.
    pub results: SimulationOutputs,
    /// Timing / resource metadata.
    pub metadata: SimulationMetadata,
}

impl SimulationResult {
    /// Sampled measurement counts.
    pub fn counts(&self) -> &HashMap<String, u64> {
        &self.results.counts
    }

    /// Per-basis-state probabilities.
    pub fn probabilities(&self) -> &HashMap<String, f64> {
        &self.results.probabilities
    }

    /// Statevector amplitudes.
    pub fn statevector(&self) -> &[Amplitude] {
        &self.results.statevector
    }

    /// The most probable basis state and its probability, if any.
    pub fn most_probable(&self) -> Option<(&str, f64)> {
        self.results
            .probabilities
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(state, &p)| (state.as_str(), p))
    }
}
