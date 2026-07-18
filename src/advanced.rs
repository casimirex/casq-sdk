//! Advanced features: quantum error correction, noise modeling, and quantum ML.
//!
//! Obtain a handle with [`crate::Client::advanced`]. These wrap the server's
//! `/advanced` API, which exposes real error-correcting codes (Steane, Shor),
//! noise channels and device models, and quantum machine-learning primitives
//! (a VQE optimizer with selectable ansatze, and quantum kernel matrices).

use crate::client::Client;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Error correction
// ---------------------------------------------------------------------------

/// Properties of a quantum error-correcting code.
#[derive(Clone, Debug, Deserialize)]
pub struct QecCode {
    /// Code identifier, e.g. `"steane"` or `"shor"`.
    pub id: String,
    /// Number of physical qubits per code block.
    #[serde(rename = "nPhysical")]
    pub n_physical: usize,
    /// Number of logical qubits encoded.
    #[serde(rename = "nLogical")]
    pub n_logical: usize,
    /// Code distance (it can correct up to `(distance - 1) / 2` errors).
    pub distance: usize,
    /// Number of stabilizer generators.
    #[serde(rename = "nStabilizers")]
    pub n_stabilizers: usize,
    /// Plain-language description of the code's correcting power.
    #[serde(rename = "errorCorrectionCapability")]
    pub error_correction_capability: String,
}

/// The result of encoding a logical state into a code block.
#[derive(Clone, Debug, Deserialize)]
pub struct EncodedState {
    /// The code used.
    pub code: String,
    /// Physical qubit count.
    #[serde(rename = "nPhysical")]
    pub n_physical: usize,
    /// Logical qubit count.
    #[serde(rename = "nLogical")]
    pub n_logical: usize,
    /// The (logical) state that was encoded.
    #[serde(rename = "logicalState")]
    pub logical_state: Vec<i64>,
    /// Stabilizer syndrome of the freshly encoded state (all zeros when clean).
    pub syndrome: Vec<i64>,
}

/// The result of a syndrome measurement.
#[derive(Clone, Debug, Deserialize)]
pub struct SyndromeResult {
    /// The code used.
    pub code: String,
    /// Measured stabilizer syndrome.
    pub syndrome: Vec<i64>,
    /// Inferred error pattern, if any.
    #[serde(rename = "errorPattern")]
    pub error_pattern: Vec<i64>,
    /// Correction the decoder would apply.
    pub correction: Vec<i64>,
}

// ---------------------------------------------------------------------------
// Noise modeling
// ---------------------------------------------------------------------------

/// The noise channels and device models the server supports.
#[derive(Clone, Debug)]
pub struct NoiseCatalog {
    /// Supported channel ids (e.g. `depolarizing`, `amplitude_damping`).
    pub channels: Vec<String>,
    /// Built-in device model ids (e.g. `ideal`, `ibmq_lagos`).
    pub models: Vec<String>,
}

#[derive(Deserialize)]
struct NoiseCatalogRaw {
    channels: Vec<IdOnly>,
    models: Vec<String>,
}

#[derive(Deserialize)]
struct IdOnly {
    id: String,
}

/// A noise channel to validate: a channel `type`, its parameters, and targets.
#[derive(Clone, Debug, Serialize)]
pub struct NoiseChannel {
    /// Channel type id (e.g. `"depolarizing"`).
    #[serde(rename = "type")]
    pub channel_type: String,
    /// Channel parameters (e.g. `{"probability": 0.01}`).
    pub params: HashMap<String, f64>,
    /// Qubits the channel acts on.
    pub targets: Vec<usize>,
}

impl NoiseChannel {
    /// Convenience constructor for a single-parameter channel on one qubit.
    pub fn new(channel_type: impl Into<String>, param: (&str, f64), target: usize) -> Self {
        let mut params = HashMap::new();
        params.insert(param.0.to_string(), param.1);
        Self {
            channel_type: channel_type.into(),
            params,
            targets: vec![target],
        }
    }
}

/// Result of validating a set of noise channels.
#[derive(Clone, Debug, Deserialize)]
pub struct NoiseValidation {
    /// Each input channel echoed back with a `valid` flag.
    pub channels: Vec<Value>,
    /// Whether every channel validated.
    #[serde(rename = "allValid")]
    pub all_valid: bool,
}

/// Characteristics generated from a device noise model.
#[derive(Clone, Debug, Deserialize)]
pub struct DeviceCharacteristics {
    /// The model queried.
    pub model: String,
    /// Raw characteristics blob (connectivity, gate times, T1/T2, error rates).
    /// Kept as a flexible value because the schema varies by model.
    pub characteristics: Value,
}

impl DeviceCharacteristics {
    /// Number of qubits in the modeled device, if present.
    pub fn n_qubits(&self) -> Option<u64> {
        self.characteristics.get("nQubits").and_then(Value::as_u64)
    }
}

// ---------------------------------------------------------------------------
// Quantum machine learning
// ---------------------------------------------------------------------------

/// A parameterized ansatz (trial circuit) template for VQE.
#[derive(Clone, Debug, Deserialize)]
pub struct Ansatz {
    /// Ansatz id, e.g. `"hardware_efficient"`.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Number of qubits.
    #[serde(rename = "nQubits")]
    pub n_qubits: usize,
    /// Number of repeated layers.
    #[serde(rename = "nLayers")]
    pub n_layers: usize,
    /// Number of tunable parameters.
    #[serde(rename = "nParams")]
    pub n_params: usize,
    /// Structural family of the ansatz.
    pub structure: String,
    /// Entanglement pattern (e.g. `"linear"`).
    pub entanglement: String,
}

/// The available ansatze and feature maps.
#[derive(Clone, Debug, Deserialize)]
pub struct MlCatalog {
    /// Ansatz templates.
    pub ansatze: Vec<Ansatz>,
    /// Feature-map ids for quantum kernels (e.g. `zz`, `pauli`).
    #[serde(rename = "featureMaps")]
    pub feature_maps: Vec<String>,
}

/// A Pauli term for the quantum-ML VQE Hamiltonian: a Pauli *string* + weight.
///
/// Note this differs from [`crate::algorithms::PauliTerm`]: here the operators
/// are a single string like `"ZZ"` spanning all qubits.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlPauliTerm {
    /// Pauli string, e.g. `"ZZ"` or `"XI"`.
    pub pauli: String,
    /// Scalar weight.
    pub coefficient: f64,
}

impl MlPauliTerm {
    /// Build a term from a Pauli string and coefficient.
    pub fn new(pauli: impl Into<String>, coefficient: f64) -> Self {
        Self {
            pauli: pauli.into(),
            coefficient,
        }
    }
}

/// The result of a quantum-ML VQE run.
#[derive(Clone, Debug, Deserialize)]
pub struct MlVqeResult {
    /// The ansatz used.
    pub ansatz: String,
    /// Lowest energy found.
    #[serde(rename = "minEnergy")]
    pub min_energy: f64,
    /// The optimized parameter vector.
    #[serde(rename = "optimalParams")]
    pub optimal_params: Vec<f64>,
    /// Optimizer iterations performed.
    pub iterations: u32,
    /// Whether the optimizer converged.
    pub converged: bool,
}

/// A computed quantum kernel (Gram) matrix.
#[derive(Clone, Debug, Deserialize)]
pub struct KernelMatrix {
    /// The feature map used.
    #[serde(rename = "featureMap")]
    pub feature_map: String,
    /// Matrix dimensions `[rows, cols]`.
    pub size: [usize; 2],
    /// The symmetric kernel matrix (`data.len() x data.len()`).
    pub matrix: Vec<Vec<f64>>,
}

/// Options for a quantum-ML VQE run.
#[derive(Clone, Debug, Default)]
pub struct VqeRunOptions {
    /// Classical optimizer id (e.g. `"COBYLA"`). `None` uses the server default.
    pub optimizer: Option<String>,
    /// Maximum optimizer iterations.
    pub max_iterations: Option<u32>,
    /// Measurement shots per energy evaluation.
    pub shots: Option<u32>,
}

/// Handle for the advanced-features API, borrowed from a [`Client`].
pub struct Advanced<'a> {
    pub(crate) client: &'a Client,
}

impl Advanced<'_> {
    // --- Error correction ---

    /// List the available error-correcting codes and their properties.
    pub async fn qec_codes(&self) -> Result<Vec<QecCode>> {
        #[derive(Deserialize)]
        struct Resp {
            codes: Vec<QecCode>,
        }
        let resp: Resp = self.client.get("/advanced/error-correction/codes").await?;
        Ok(resp.codes)
    }

    /// Encode a logical state with the chosen code. `logical_state` has one
    /// entry per logical qubit (pass `None` for the all-zero state).
    pub async fn encode(
        &self,
        code_id: &str,
        logical_state: Option<&[i64]>,
    ) -> Result<EncodedState> {
        let body = match logical_state {
            Some(s) => serde_json::json!({ "logicalState": s }),
            None => serde_json::json!({}),
        };
        self.client
            .post(
                &format!("/advanced/error-correction/{code_id}/encode"),
                &body,
            )
            .await
    }

    /// Measure the stabilizer syndrome of an encoded logical state.
    pub async fn syndrome(
        &self,
        code_id: &str,
        logical_state: Option<&[i64]>,
    ) -> Result<SyndromeResult> {
        let mut body = serde_json::json!({ "code": code_id });
        if let Some(s) = logical_state {
            body["logicalState"] = serde_json::json!(s);
        }
        self.client
            .post("/advanced/error-correction/syndrome", &body)
            .await
    }

    // --- Noise ---

    /// List supported noise channels and built-in device models.
    pub async fn noise_catalog(&self) -> Result<NoiseCatalog> {
        let raw: NoiseCatalogRaw = self.client.get("/advanced/noise/channels").await?;
        Ok(NoiseCatalog {
            channels: raw.channels.into_iter().map(|c| c.id).collect(),
            models: raw.models,
        })
    }

    /// Validate a set of noise channels against the model.
    pub async fn validate_noise(&self, channels: &[NoiseChannel]) -> Result<NoiseValidation> {
        let body = serde_json::json!({ "channels": channels });
        self.client.post("/advanced/noise/apply", &body).await
    }

    /// Generate device characteristics from a built-in noise model.
    pub async fn characterize(&self, model: &str) -> Result<DeviceCharacteristics> {
        let body = serde_json::json!({ "model": model });
        self.client
            .post("/advanced/noise/characterize", &body)
            .await
    }

    // --- Quantum ML ---

    /// List the available VQE ansatze and quantum-kernel feature maps.
    pub async fn ml_catalog(&self) -> Result<MlCatalog> {
        self.client.get("/advanced/ml/vqe/ansatz").await
    }

    /// Run a quantum-ML VQE optimization over `hamiltonian` with `ansatz`.
    pub async fn ml_vqe(
        &self,
        hamiltonian: &[MlPauliTerm],
        ansatz: &str,
        options: VqeRunOptions,
    ) -> Result<MlVqeResult> {
        let mut body = serde_json::json!({
            "hamiltonian": hamiltonian,
            "ansatz": ansatz,
        });
        if let Some(o) = options.optimizer {
            body["optimizer"] = serde_json::json!(o);
        }
        if let Some(m) = options.max_iterations {
            body["maxIterations"] = serde_json::json!(m);
        }
        if let Some(s) = options.shots {
            body["shots"] = serde_json::json!(s);
        }
        self.client.post("/advanced/ml/vqe/run", &body).await
    }

    /// Compute the quantum kernel matrix for `data` using `feature_map`
    /// (pass `None` for the server default feature map).
    pub async fn kernel_matrix(
        &self,
        data: &[Vec<f64>],
        feature_map: Option<&str>,
    ) -> Result<KernelMatrix> {
        let mut body = serde_json::json!({ "data": data });
        if let Some(fm) = feature_map {
            body["featureMap"] = serde_json::json!(fm);
        }
        self.client.post("/advanced/ml/kernel/matrix", &body).await
    }
}
