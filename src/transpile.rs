//! Transpilation: rewrite a circuit into a device's native gate basis.
//!
//! Real hardware executes only a fixed set of gates, so a circuit must be
//! *transpiled* — decomposed into that basis — before it can run. Call
//! [`crate::Client::transpile`] to get the native circuit plus stats.

use crate::circuit::{Circuit, Operation};
use serde::Deserialize;

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
