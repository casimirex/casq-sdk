//! Circuit construction.
//!
//! A [`Circuit`] is built locally with a fluent, Qiskit-style gate API and then
//! sent to the server for simulation via [`crate::Client::run`]. Operations are
//! serialized in the shape the casimirQ API expects: `{ gate, targets, params? }`,
//! where controls are folded into `targets` (e.g. `cx` → `targets: [control, target]`).

use serde::{Deserialize, Serialize};

/// A single gate/operation applied to a set of target qubits.
///
/// `targets` holds every qubit the operation touches, control qubits first
/// (matching the positional arguments of the server-side circuit builder).
/// `params` carries continuous parameters such as rotation angles.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    /// Gate name, e.g. `"h"`, `"cx"`, `"rx"`. Aliases such as `"cnot"` are
    /// accepted by the server.
    pub gate: String,
    /// Qubit indices the gate acts on (controls first).
    pub targets: Vec<usize>,
    /// Continuous parameters (e.g. a rotation angle), omitted when empty.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub params: Option<Vec<f64>>,
}

impl Operation {
    /// Construct an operation with no continuous parameters.
    pub fn new(gate: impl Into<String>, targets: impl Into<Vec<usize>>) -> Self {
        Self {
            gate: gate.into(),
            targets: targets.into(),
            params: None,
        }
    }

    /// Construct an operation carrying continuous parameters.
    pub fn with_params(
        gate: impl Into<String>,
        targets: impl Into<Vec<usize>>,
        params: impl Into<Vec<f64>>,
    ) -> Self {
        Self {
            gate: gate.into(),
            targets: targets.into(),
            params: Some(params.into()),
        }
    }
}

/// A quantum circuit: a qubit count plus an ordered list of operations.
///
/// # Example
/// ```
/// use casq_sdk::Circuit;
///
/// let mut circuit = Circuit::new(2);
/// circuit.h(0).cx(0, 1); // Bell state
/// assert_eq!(circuit.operations().len(), 2);
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Circuit {
    #[serde(rename = "numQubits")]
    num_qubits: usize,
    operations: Vec<Operation>,
}

impl Circuit {
    /// Create an empty circuit over `num_qubits` qubits.
    pub fn new(num_qubits: usize) -> Self {
        Self {
            num_qubits,
            operations: Vec::new(),
        }
    }

    /// Number of qubits in the circuit.
    pub fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    /// The operations added so far, in order.
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    /// Append a raw [`Operation`]. Escape hatch for gates without a dedicated
    /// method.
    pub fn push(&mut self, op: Operation) -> &mut Self {
        self.operations.push(op);
        self
    }

    fn add(&mut self, gate: &str, targets: Vec<usize>) -> &mut Self {
        self.operations.push(Operation::new(gate, targets));
        self
    }

    fn add_p(&mut self, gate: &str, targets: Vec<usize>, params: Vec<f64>) -> &mut Self {
        self.operations
            .push(Operation::with_params(gate, targets, params));
        self
    }

    // --- Single-qubit gates ---

    /// Hadamard gate.
    pub fn h(&mut self, q: usize) -> &mut Self {
        self.add("h", vec![q])
    }
    /// Pauli-X (NOT) gate.
    pub fn x(&mut self, q: usize) -> &mut Self {
        self.add("x", vec![q])
    }
    /// Pauli-Y gate.
    pub fn y(&mut self, q: usize) -> &mut Self {
        self.add("y", vec![q])
    }
    /// Pauli-Z gate.
    pub fn z(&mut self, q: usize) -> &mut Self {
        self.add("z", vec![q])
    }
    /// S (phase) gate.
    pub fn s(&mut self, q: usize) -> &mut Self {
        self.add("s", vec![q])
    }
    /// S-dagger gate.
    pub fn sdg(&mut self, q: usize) -> &mut Self {
        self.add("sdg", vec![q])
    }
    /// T gate.
    pub fn t(&mut self, q: usize) -> &mut Self {
        self.add("t", vec![q])
    }
    /// T-dagger gate.
    pub fn tdg(&mut self, q: usize) -> &mut Self {
        self.add("tdg", vec![q])
    }

    // --- Rotations / phase ---

    /// Rotation about the X axis by `theta` radians.
    pub fn rx(&mut self, q: usize, theta: f64) -> &mut Self {
        self.add_p("rx", vec![q], vec![theta])
    }
    /// Rotation about the Y axis by `theta` radians.
    pub fn ry(&mut self, q: usize, theta: f64) -> &mut Self {
        self.add_p("ry", vec![q], vec![theta])
    }
    /// Rotation about the Z axis by `theta` radians.
    pub fn rz(&mut self, q: usize, theta: f64) -> &mut Self {
        self.add_p("rz", vec![q], vec![theta])
    }
    /// Phase gate by `lambda` radians.
    pub fn p(&mut self, q: usize, lambda: f64) -> &mut Self {
        self.add_p("p", vec![q], vec![lambda])
    }

    // --- Two-qubit gates ---

    /// Controlled-X (CNOT).
    pub fn cx(&mut self, control: usize, target: usize) -> &mut Self {
        self.add("cx", vec![control, target])
    }
    /// Alias for [`Circuit::cx`].
    pub fn cnot(&mut self, control: usize, target: usize) -> &mut Self {
        self.cx(control, target)
    }
    /// Controlled-Y.
    pub fn cy(&mut self, control: usize, target: usize) -> &mut Self {
        self.add("cy", vec![control, target])
    }
    /// Controlled-Z.
    pub fn cz(&mut self, control: usize, target: usize) -> &mut Self {
        self.add("cz", vec![control, target])
    }
    /// Controlled-Hadamard.
    pub fn ch(&mut self, control: usize, target: usize) -> &mut Self {
        self.add("ch", vec![control, target])
    }
    /// SWAP two qubits.
    pub fn swap(&mut self, a: usize, b: usize) -> &mut Self {
        self.add("swap", vec![a, b])
    }
    /// Controlled phase rotation by `lambda` radians.
    pub fn cp(&mut self, control: usize, target: usize, lambda: f64) -> &mut Self {
        self.add_p("cp", vec![control, target], vec![lambda])
    }

    /// Controlled X-rotation: `Rx(theta)` on `target` when `control` is set.
    pub fn crx(&mut self, control: usize, target: usize, theta: f64) -> &mut Self {
        self.add_p("crx", vec![control, target], vec![theta])
    }

    /// Controlled Y-rotation: `Ry(theta)` on `target` when `control` is set.
    pub fn cry(&mut self, control: usize, target: usize, theta: f64) -> &mut Self {
        self.add_p("cry", vec![control, target], vec![theta])
    }

    /// Controlled Z-rotation: `Rz(theta)` on `target` when `control` is set.
    pub fn crz(&mut self, control: usize, target: usize, theta: f64) -> &mut Self {
        self.add_p("crz", vec![control, target], vec![theta])
    }

    // --- Three-qubit gates ---

    /// Toffoli (CCX): X on `target` when both controls are set.
    pub fn ccx(&mut self, control1: usize, control2: usize, target: usize) -> &mut Self {
        self.add("ccx", vec![control1, control2, target])
    }
    /// Alias for [`Circuit::ccx`].
    pub fn toffoli(&mut self, control1: usize, control2: usize, target: usize) -> &mut Self {
        self.ccx(control1, control2, target)
    }
    /// Fredkin (controlled-SWAP).
    pub fn cswap(&mut self, control: usize, target1: usize, target2: usize) -> &mut Self {
        self.add("cswap", vec![control, target1, target2])
    }

    // --- Multi-controlled gates ---
    // Controls are folded into the target list ([...controls, target]), matching
    // the API's convention for ccx/cswap.

    /// Multi-controlled X (generalized CNOT/Toffoli): flip `target` when every
    /// qubit in `controls` is set.
    pub fn mcx(&mut self, controls: &[usize], target: usize) -> &mut Self {
        let mut targets = controls.to_vec();
        targets.push(target);
        self.add("mcx", targets)
    }

    /// Multi-controlled Z: phase-flip `|1…1⟩` over `controls` + `target`.
    pub fn mcz(&mut self, controls: &[usize], target: usize) -> &mut Self {
        let mut targets = controls.to_vec();
        targets.push(target);
        self.add("mcz", targets)
    }

    /// Doubly-controlled Z (a 2-control [`Circuit::mcz`]).
    pub fn ccz(&mut self, control1: usize, control2: usize, target: usize) -> &mut Self {
        self.add("ccz", vec![control1, control2, target])
    }

    // --- Non-unitary / structural ---

    /// Measure a single qubit.
    pub fn measure(&mut self, q: usize) -> &mut Self {
        self.add("measure", vec![q])
    }

    /// Measure every qubit in the circuit.
    pub fn measure_all(&mut self) -> &mut Self {
        for q in 0..self.num_qubits {
            self.add("measure", vec![q]);
        }
        self
    }
}
