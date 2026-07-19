//! Offline unit tests exercising serialization / deserialization against the
//! exact JSON shapes the casimirQ API uses. These need no server.

use casq_sdk::advanced::{
    KernelMatrix, MlPauliTerm, MlVqeResult, NoiseChannel, NoiseChannelConfig,
    NoiseSimulationResult, QecCode,
};
use casq_sdk::backends::{Backend, BackendRunResult};
use casq_sdk::jobs::{Job, JobStatus};
use casq_sdk::TranspileResult;
use casq_sdk::{Circuit, Engine, Operation, RunOptions, SimulationResult};

#[test]
fn bell_circuit_serializes_to_api_operations() {
    let mut circuit = Circuit::new(2);
    circuit.h(0).cx(0, 1);

    assert_eq!(circuit.num_qubits(), 2);
    assert_eq!(
        circuit.operations(),
        &[
            Operation::new("h", vec![0]),
            Operation::new("cx", vec![0, 1])
        ]
    );

    let json = serde_json::to_value(&circuit).unwrap();
    assert_eq!(json["numQubits"], 2);
    assert_eq!(json["operations"][0]["gate"], "h");
    assert_eq!(json["operations"][0]["targets"][0], 0);
    // No params key emitted for a plain gate.
    assert!(json["operations"][0].get("params").is_none());
    assert_eq!(json["operations"][1]["gate"], "cx");
    assert_eq!(json["operations"][1]["targets"], serde_json::json!([0, 1]));
}

#[test]
fn rotation_gate_emits_params() {
    let mut circuit = Circuit::new(1);
    circuit.rx(0, std::f64::consts::FRAC_PI_2);

    let json = serde_json::to_value(&circuit).unwrap();
    assert_eq!(json["operations"][0]["gate"], "rx");
    assert_eq!(json["operations"][0]["targets"], serde_json::json!([0]));
    assert_eq!(
        json["operations"][0]["params"][0].as_f64().unwrap(),
        std::f64::consts::FRAC_PI_2
    );
}

#[test]
fn aliases_map_to_canonical_gates() {
    let mut c = Circuit::new(3);
    c.cnot(0, 1).toffoli(0, 1, 2);
    assert_eq!(c.operations()[0].gate, "cx");
    assert_eq!(c.operations()[1].gate, "ccx");
    assert_eq!(c.operations()[1].targets, vec![0, 1, 2]);
}

#[test]
fn measure_all_adds_one_measure_per_qubit() {
    let mut c = Circuit::new(3);
    c.measure_all();
    assert_eq!(c.operations().len(), 3);
    assert!(c.operations().iter().all(|o| o.gate == "measure"));
}

#[test]
fn engine_serializes_lowercase() {
    assert_eq!(
        serde_json::to_value(Engine::Statevector).unwrap(),
        "statevector"
    );
    assert_eq!(serde_json::to_value(Engine::Clifford).unwrap(), "clifford");
    assert_eq!(serde_json::to_value(Engine::Auto).unwrap(), "auto");
    assert_eq!(Engine::default(), Engine::Auto);
}

#[test]
fn run_options_builder() {
    let opts = RunOptions::new().engine(Engine::Mps).shots(2048);
    let json = serde_json::to_value(&opts).unwrap();
    assert_eq!(json["engine"], "mps");
    assert_eq!(json["shots"], 2048);

    // Default omits shots.
    let json = serde_json::to_value(RunOptions::new()).unwrap();
    assert!(json.get("shots").is_none());
}

#[test]
fn simulation_result_parses_bell_response() {
    // Exactly the shape POST /circuits/:id/simulate returns.
    let raw = r#"{
        "circuitId": "circuit-1",
        "jobId": "sim-1",
        "status": "completed",
        "numQubits": 2,
        "requestedEngine": "statevector",
        "shots": 1024,
        "results": {
            "statevector": [
                {"state":"00","re":0.7071,"im":0.0,"probability":0.5},
                {"state":"11","re":0.7071,"im":0.0,"probability":0.5}
            ],
            "probabilities": {"00":0.5,"11":0.5},
            "counts": {"00":512,"11":512}
        },
        "metadata": {"executionTimeMs":0.08,"memoryUsageBytes":3888}
    }"#;

    let result: SimulationResult = serde_json::from_str(raw).unwrap();
    assert_eq!(result.num_qubits, 2);
    assert_eq!(result.job_id, "sim-1");
    assert_eq!(result.counts()["00"], 512);
    assert_eq!(result.statevector().len(), 2);
    assert_eq!(result.metadata.memory_usage_bytes, 3888);

    let (state, prob) = result.most_probable().unwrap();
    assert!(state == "00" || state == "11");
    assert!((prob - 0.5).abs() < 1e-9);
}

#[test]
fn qec_code_parses() {
    let raw = r#"{
        "id":"steane","nPhysical":7,"nLogical":1,"distance":3,
        "nStabilizers":6,"errorCorrectionCapability":"Can correct any 1-qubit error"
    }"#;
    let code: QecCode = serde_json::from_str(raw).unwrap();
    assert_eq!(code.id, "steane");
    assert_eq!(code.n_physical, 7);
    assert_eq!(code.n_logical, 1);
    assert_eq!(code.distance, 3);
}

#[test]
fn ml_vqe_result_parses() {
    let raw = r#"{
        "ansatz":"hardware_efficient","minEnergy":-0.5,
        "optimalParams":[0.1,0.2,0.3],"iterations":3,"converged":true
    }"#;
    let r: MlVqeResult = serde_json::from_str(raw).unwrap();
    assert_eq!(r.min_energy, -0.5);
    assert_eq!(r.optimal_params.len(), 3);
    assert!(r.converged);
}

#[test]
fn kernel_matrix_parses() {
    let raw = r#"{"featureMap":"zz","size":[2,2],"matrix":[[1.0,0.8],[0.8,1.0]]}"#;
    let k: KernelMatrix = serde_json::from_str(raw).unwrap();
    assert_eq!(k.feature_map, "zz");
    assert_eq!(k.size, [2, 2]);
    assert_eq!(k.matrix[0][1], 0.8);
}

#[test]
fn ml_pauli_term_serializes() {
    let term = MlPauliTerm::new("ZZ", 1.5);
    let json = serde_json::to_value(&term).unwrap();
    assert_eq!(json["pauli"], "ZZ");
    assert_eq!(json["coefficient"], 1.5);
}

#[test]
fn noise_channel_serializes_with_type_rename() {
    let ch = NoiseChannel::new("depolarizing", ("probability", 0.01), 0);
    let json = serde_json::to_value(&ch).unwrap();
    assert_eq!(json["type"], "depolarizing");
    assert_eq!(json["params"]["probability"], 0.01);
    assert_eq!(json["targets"], serde_json::json!([0]));
}

#[test]
fn noise_channel_config_serializes_with_type_and_params() {
    let depol = NoiseChannelConfig::depolarizing(0.1);
    let j = serde_json::to_value(&depol).unwrap();
    assert_eq!(j["type"], "depolarizing");
    assert_eq!(j["params"]["p"], 0.1);
    // Only the relevant param is present.
    assert!(j["params"].get("gamma").is_none());

    let ad = NoiseChannelConfig::amplitude_damping(0.25);
    let j = serde_json::to_value(&ad).unwrap();
    assert_eq!(j["type"], "amplitude_damping");
    assert_eq!(j["params"]["gamma"], 0.25);
    assert!(j["params"].get("p").is_none());
}

#[test]
fn noise_simulation_result_parses() {
    let raw = r#"{
        "engine":"density-matrix","numQubits":2,"purity":0.68,"fidelity":0.82,
        "probabilities":{"00":0.45,"11":0.45,"01":0.05,"10":0.05},
        "counts":{"00":450,"11":450,"01":50,"10":50},
        "executionTimeMs":1.2
    }"#;
    let r: NoiseSimulationResult = serde_json::from_str(raw).unwrap();
    assert_eq!(r.engine, "density-matrix");
    assert_eq!(r.num_qubits, 2);
    assert_eq!(r.fidelity, Some(0.82));
    assert_eq!(r.counts["00"], 450);
    assert!(r.purity < 1.0);
}

#[test]
fn noise_simulation_result_without_fidelity() {
    let raw = r#"{"engine":"density-matrix","numQubits":1,"purity":0.5,
        "probabilities":{"0":0.5,"1":0.5},"counts":{"0":500,"1":500},"executionTimeMs":0.3}"#;
    let r: NoiseSimulationResult = serde_json::from_str(raw).unwrap();
    assert_eq!(r.fidelity, None);
}

#[test]
fn backend_parses_with_capabilities() {
    let raw = r#"{
        "id":"emulated-qpu","name":"Emulated QPU","type":"hardware-emulator",
        "description":"emulated","available":true,
        "capabilities":{"maxQubits":7,"nativeGates":["rz","sx","x","cx"],
            "supportsNoise":true,"connectivity":"linear","simulated":true}
    }"#;
    let b: Backend = serde_json::from_str(raw).unwrap();
    assert_eq!(b.id, "emulated-qpu");
    assert_eq!(b.backend_type, "hardware-emulator");
    assert_eq!(b.capabilities.max_qubits, 7);
    assert_eq!(b.capabilities.connectivity, "linear");
    assert!(!b.capabilities.native_gates.contains(&"h".to_string()));
}

#[test]
fn backend_run_result_exposes_metadata_accessors() {
    let raw = r#"{
        "backendId":"emulated-qpu","numQubits":2,"shots":1000,
        "counts":{"00":480,"11":506,"01":7,"10":7},
        "probabilities":{"00":0.48,"11":0.506,"01":0.007,"10":0.007},
        "metadata":{"executionTimeMs":0.7,"purity":0.9609,"nativeGateFraction":0.5,"emulated":true}
    }"#;
    let r: BackendRunResult = serde_json::from_str(raw).unwrap();
    assert_eq!(r.backend_id, "emulated-qpu");
    assert_eq!(r.counts["00"], 480);
    assert_eq!(r.execution_time_ms(), Some(0.7));
    assert_eq!(r.purity(), Some(0.9609));
    assert_eq!(r.native_gate_fraction(), Some(0.5));
}

#[test]
fn job_status_terminality() {
    assert!(JobStatus::Completed.is_terminal());
    assert!(JobStatus::Failed.is_terminal());
    assert!(JobStatus::Cancelled.is_terminal());
    assert!(!JobStatus::Queued.is_terminal());
    assert!(!JobStatus::Running.is_terminal());
}

#[test]
fn queued_job_parses_without_result() {
    let raw = r#"{
        "id":"job-1","type":"simulation","status":"queued","progress":0,
        "result":null,"error":null,
        "createdAt":"t","updatedAt":"t","startedAt":null,"finishedAt":null
    }"#;
    let job: Job = serde_json::from_str(raw).unwrap();
    assert_eq!(job.status, JobStatus::Queued);
    assert!(job.result.is_none());
    assert!(job.finished_at.is_none());
}

#[test]
fn completed_job_exposes_result_and_backend() {
    let raw = r#"{
        "id":"job-2","type":"simulation","status":"completed","progress":1,
        "result":{"status":"completed","numQubits":2,"requestedEngine":"emulated-qpu","shots":2000,
            "results":{"statevector":[],"probabilities":{"00":0.5,"11":0.5},"counts":{"00":1000,"11":1000}},
            "metadata":{"executionTimeMs":0.7,"backendId":"emulated-qpu","purity":0.961}},
        "error":null,"createdAt":"t","updatedAt":"t","startedAt":"t","finishedAt":"t"
    }"#;
    let job: Job = serde_json::from_str(raw).unwrap();
    assert_eq!(job.status, JobStatus::Completed);
    let result = job.result.unwrap();
    assert_eq!(result.requested_engine, "emulated-qpu");
    assert_eq!(result.counts()["00"], 1000);
    assert_eq!(result.backend_id(), Some("emulated-qpu"));
    assert_eq!(result.execution_time_ms(), Some(0.7));
}

#[test]
fn transpile_result_parses_and_builds_a_circuit() {
    let raw = r#"{
        "operations":[
            {"gate":"rz","targets":[0],"params":[3.14159]},
            {"gate":"ry","targets":[0],"params":[1.5708]},
            {"gate":"cx","targets":[0,1]}
        ],
        "basis":["id","rz","ry","cx"],
        "originalGateCount":2,"transpiledGateCount":3,
        "fullyNative":true,"unsupported":[]
    }"#;
    let r: TranspileResult = serde_json::from_str(raw).unwrap();
    assert!(r.fully_native);
    assert_eq!(r.original_gate_count, 2);
    assert_eq!(r.transpiled_gate_count, 3);
    assert_eq!(r.basis, vec!["id", "rz", "ry", "cx"]);

    // Every operation is in the native basis, and it rebuilds into a Circuit.
    for op in &r.operations {
        assert!(r.basis.contains(&op.gate));
    }
    let circuit = r.to_circuit(2);
    assert_eq!(circuit.num_qubits(), 2);
    assert_eq!(circuit.operations().len(), 3);

    // A plain (unrouted) result has no routing fields.
    assert!(r.final_permutation.is_none());
    assert!(r.swap_count.is_none());
}

#[test]
fn routed_transpile_result_parses_permutation_and_swaps() {
    let raw = r#"{
        "operations":[{"gate":"cx","targets":[0,1]}],
        "basis":["id","rz","ry","cx"],
        "originalGateCount":1,"transpiledGateCount":4,
        "fullyNative":true,"unsupported":[],
        "finalPermutation":[0,2,1],"swapCount":1
    }"#;
    let r: TranspileResult = serde_json::from_str(raw).unwrap();
    assert_eq!(r.final_permutation.as_deref(), Some([0, 2, 1].as_slice()));
    assert_eq!(r.swap_count, Some(1));
}
