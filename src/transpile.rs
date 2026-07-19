//! Transpilation: rewrite a circuit into a device's native gate basis, and
//! optionally route it onto the device's qubit connectivity.
//!
//! Real hardware executes only a fixed set of gates *between physically-coupled
//! qubits*, so a circuit must be *transpiled* — decomposed into that basis and,
//! when a connectivity is given, routed with SWAPs — before it can run. Call
//! [`crate::Client::transpile`] for plain decomposition, or
//! [`crate::Client::transpile_with`] to also route onto a coupling graph.

use crate::circuit::{Circuit, Operation};
use serde::Deserialize;

/// A device qubit connectivity to route onto.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Connectivity {
    /// Every qubit couples to every other — no routing needed.
    AllToAll,
    /// A line `0—1—2—…` — the emulated QPU's topology.
    Linear,
}

impl Connectivity {
    fn as_str(self) -> &'static str {
        match self {
            Connectivity::AllToAll => "all-to-all",
            Connectivity::Linear => "linear",
        }
    }
}

/// Initial-placement strategy used before routing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Layout {
    /// Start from the identity placement (logical `i` on physical `i`).
    #[default]
    Trivial,
    /// Seat interacting qubits near each other to cut SWAPs (greedy heuristic).
    Greedy,
}

impl Layout {
    fn as_str(self) -> &'static str {
        match self {
            Layout::Trivial => "trivial",
            Layout::Greedy => "greedy",
        }
    }
}

/// Options controlling transpilation. `Default` decomposes without routing.
#[derive(Clone, Debug, Default)]
pub struct TranspileOptions {
    /// Route onto this connectivity, inserting SWAPs so every two-qubit gate
    /// acts on coupled qubits.
    pub connectivity: Option<Connectivity>,
    /// An explicit coupling map (`[[a, b], ...]` undirected edges) for
    /// non-linear topologies. Takes precedence over `connectivity`.
    pub coupling: Option<Vec<[usize; 2]>>,
    /// Initial-placement strategy when routing (default [`Layout::Trivial`]).
    pub layout: Option<Layout>,
}

impl TranspileOptions {
    /// Route onto the given connectivity.
    pub fn connectivity(connectivity: Connectivity) -> Self {
        Self {
            connectivity: Some(connectivity),
            coupling: None,
            layout: None,
        }
    }

    /// Route onto an explicit coupling map.
    pub fn coupling(edges: impl Into<Vec<[usize; 2]>>) -> Self {
        Self {
            connectivity: None,
            coupling: Some(edges.into()),
            layout: None,
        }
    }

    /// Set the initial-placement strategy (chainable).
    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Serialize into the request-body fields the API expects.
    pub(crate) fn apply(&self, body: &mut serde_json::Value) {
        if let Some(c) = self.connectivity {
            body["connectivity"] = serde_json::json!(c.as_str());
        }
        if let Some(edges) = &self.coupling {
            body["coupling"] = serde_json::json!(edges);
        }
        if let Some(layout) = self.layout {
            body["layout"] = serde_json::json!(layout.as_str());
        }
    }
}

/// The result of transpiling a circuit into a native gate basis.
#[derive(Clone, Debug, Deserialize)]
pub struct TranspileResult {
    /// The circuit rewritten into native gates.
    pub operations: Vec<Operation>,
    /// The native basis targeted (e.g. `["id", "rz", "ry", "cx"]`).
    pub basis: Vec<String>,
    /// Operation count before transpilation.
    #[serde(rename = "originalGateCount")]
    pub original_gate_count: usize,
    /// Operation count after transpilation (often larger — decomposition cost).
    #[serde(rename = "transpiledGateCount")]
    pub transpiled_gate_count: usize,
    /// Whether every operation is now in the native basis.
    #[serde(rename = "fullyNative")]
    pub fully_native: bool,
    /// Gate types that could not be decomposed (passed through unchanged).
    pub unsupported: Vec<String>,
    /// Present when the circuit was routed onto a connectivity:
    /// `final_permutation[logical] = physical` qubit that holds it afterwards.
    /// Read a measurement of logical qubit `l` from physical
    /// `final_permutation[l]`.
    #[serde(rename = "finalPermutation")]
    pub final_permutation: Option<Vec<usize>>,
    /// The chosen initial placement: `initial_layout[logical] = physical` qubit
    /// it started on. Prepare an input for logical qubit `l` on physical
    /// `initial_layout[l]`.
    #[serde(rename = "initialLayout")]
    pub initial_layout: Option<Vec<usize>>,
    /// Number of SWAPs inserted by routing (each expands to `3×cx`).
    #[serde(rename = "swapCount")]
    pub swap_count: Option<usize>,
}

impl TranspileResult {
    /// Build a runnable [`Circuit`] from the transpiled operations.
    pub fn to_circuit(&self, num_qubits: usize) -> Circuit {
        let mut circuit = Circuit::new(num_qubits);
        for op in &self.operations {
            circuit.push(op.clone());
        }
        circuit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_serialize_into_the_request_body() {
        let mut body = serde_json::json!({ "numQubits": 3 });
        TranspileOptions::connectivity(Connectivity::Linear).apply(&mut body);
        assert_eq!(body["connectivity"], "linear");

        let mut body = serde_json::json!({ "numQubits": 4 });
        TranspileOptions::coupling(vec![[0, 1], [0, 2]]).apply(&mut body);
        assert_eq!(body["coupling"], serde_json::json!([[0, 1], [0, 2]]));

        // A layout strategy serializes alongside the connectivity.
        let mut body = serde_json::json!({ "numQubits": 3 });
        TranspileOptions::connectivity(Connectivity::Linear)
            .with_layout(Layout::Greedy)
            .apply(&mut body);
        assert_eq!(body["connectivity"], "linear");
        assert_eq!(body["layout"], "greedy");

        // Default routes nothing.
        let mut body = serde_json::json!({ "numQubits": 2 });
        TranspileOptions::default().apply(&mut body);
        assert!(body.get("connectivity").is_none());
        assert!(body.get("coupling").is_none());
        assert!(body.get("layout").is_none());
    }
}
