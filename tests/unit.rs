//! Offline unit tests exercising serialization / deserialization against the
//! exact JSON shapes the casimirQ API uses. These need no server.

use casq_sdk::advanced::{KernelMatrix, MlPauliTerm, MlVqeResult, NoiseChannel, QecCode};
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
