//! End-to-end tests against a live casimirQ server.
//!
//! These are skipped unless `CASQ_BASE_URL` is set, so the default `cargo test`
//! run needs no server. To run them:
//! ```bash
//! CASQ_BASE_URL=http://localhost:8080/api/v1 \
//! CASQ_EMAIL=admin@example.com CASQ_PASSWORD=admin123 \
//!   cargo test --test integration -- --nocapture
//! ```

use casq_sdk::{Circuit, Client, Engine, RunOptions};

struct Config {
    base_url: String,
    email: String,
    password: String,
}

/// Returns the server config, or `None` when integration tests should be skipped.
fn config() -> Option<Config> {
    let base_url = std::env::var("CASQ_BASE_URL").ok()?;
    Some(Config {
        base_url,
        email: std::env::var("CASQ_EMAIL").unwrap_or_else(|_| "admin@example.com".into()),
        password: std::env::var("CASQ_PASSWORD").unwrap_or_else(|_| "admin123".into()),
    })
}

async fn authed_client(cfg: &Config) -> Client {
    let mut client = Client::new(&cfg.base_url).expect("valid base url");
    client
        .login(&cfg.email, &cfg.password)
        .await
        .expect("login should succeed");
    assert!(client.is_authenticated());
    client
}

#[tokio::test]
async fn unauthenticated_call_is_rejected_locally() {
    // No network needed: the client refuses protected calls without a token.
    let client = Client::new("http://localhost:8080/api/v1").unwrap();
    let err = client.list_circuits(1, 10).await.unwrap_err();
    assert!(matches!(err, casq_sdk::Error::NotAuthenticated));
}

#[tokio::test]
async fn bell_state_runs_and_samples_correctly() {
    let Some(cfg) = config() else {
        eprintln!("skipping: set CASQ_BASE_URL to run integration tests");
        return;
    };
    let client = authed_client(&cfg).await;

    let mut circuit = Circuit::new(2);
    circuit.h(0).cx(0, 1);

    let shots = 1024;
    let result = client
        .run(
            &circuit,
            RunOptions::new().engine(Engine::Statevector).shots(shots),
        )
        .await
        .expect("run should succeed");

    assert_eq!(result.status, "completed");
    assert_eq!(result.num_qubits, 2);
    // A Bell state only ever collapses to |00> or |11>.
    for state in result.counts().keys() {
        assert!(state == "00" || state == "11", "unexpected state {state}");
    }
    let total: u64 = result.counts().values().sum();
    assert_eq!(total, shots as u64);
}

#[tokio::test]
async fn grover_amplifies_the_marked_item() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    let result = client
        .algorithms()
        .grover(4, 9, None)
        .await
        .expect("grover");
    assert!(
        result.success_probability > 0.9,
        "expected amplification, got {}",
        result.success_probability
    );
}

#[tokio::test]
async fn shor_factors_fifteen() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    let result = client.algorithms().shor(15).await.expect("shor");
    let mut factors = result.factors.clone();
    factors.sort_unstable();
    assert_eq!(factors, vec![3, 5]);
    // Genuine quantum order finding: the period is the multiplicative order.
    assert_eq!(result.period, 4);
    assert!(result.base.is_some());
}

#[tokio::test]
async fn oracle_algorithms_recover_their_secrets() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;
    let algos = client.algorithms();

    let dj = algos
        .deutsch_jozsa(3, "balanced", None, Some(5))
        .await
        .expect("deutsch-jozsa");
    assert_eq!(dj.decision, "balanced");
    assert!(dj.correct);

    let bv = algos
        .bernstein_vazirani(5, 21)
        .await
        .expect("bernstein-vazirani");
    assert_eq!(bv.recovered, 21);
    assert!(bv.correct);

    let simon = algos.simon(3, 6).await.expect("simon");
    assert_eq!(simon.recovered, 6);
    assert!(simon.correct);
}

#[tokio::test]
async fn phase_estimation_recovers_a_dyadic_phase() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    let qpe = client
        .algorithms()
        .phase_estimation(0.375, 4)
        .await
        .expect("phase-estimation");
    assert_eq!(qpe.measured_integer, 6);
    assert!((qpe.estimated_phase - 0.375).abs() < 1e-9);
}

#[tokio::test]
async fn amplitude_amplification_amplifies_a_good_state() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    let aa = client
        .algorithms()
        .amplitude_amplification(&[std::f64::consts::FRAC_PI_2; 3], &[5], None)
        .await
        .expect("amplitude-amplification");
    assert!(aa.final_probability > aa.initial_probability);
    assert!(aa.final_probability > 0.9);
}

#[tokio::test]
async fn quantum_walk_spreads_ballistically() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    let walk = client
        .algorithms()
        .quantum_walk(5, 8, None, None)
        .await
        .expect("quantum-walk");
    assert!(walk.std_dev > walk.classical_std_dev);
    assert!(!walk.distribution.is_empty());
}

#[tokio::test]
async fn hamiltonian_simulation_evolves_under_x() {
    use casq_sdk::algorithms::PauliTerm;
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    let t = 0.7_f64;
    let terms = [PauliTerm {
        coefficient: 1.0,
        paulis: vec!["X".into()],
        qubits: vec![0],
    }];
    let sim = client
        .algorithms()
        .hamiltonian_simulation(1, &terms, t, None, None, None)
        .await
        .expect("hamiltonian-simulation");
    let p1 = sim
        .probabilities
        .iter()
        .find(|p| p.state == 1)
        .map(|p| p.probability)
        .unwrap_or(0.0);
    assert!((p1 - t.sin().powi(2)).abs() < 1e-6);
}

#[tokio::test]
async fn hhl_solves_a_linear_system() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    let hhl = client.algorithms().hhl(1.0, 0.0).await.expect("hhl");
    assert!(hhl.fidelity > 0.99);
    assert_eq!(hhl.classical_solution.len(), 2);
}

#[tokio::test]
async fn circuit_persistence_roundtrip() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    let mut circuit = Circuit::new(2);
    circuit.h(0).cx(0, 1);

    let created = client
        .create_circuit("sdk-integration-test", &circuit)
        .await
        .expect("create");
    assert_eq!(created.num_qubits, 2);

    let fetched = client.get_circuit(&created.id).await.expect("get");
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.operations.len(), 2);

    let list = client.list_circuits(1, 50).await.expect("list");
    assert!(list.circuits.iter().any(|c| c.id == created.id));

    client.delete_circuit(&created.id).await.expect("delete");
    assert!(client.get_circuit(&created.id).await.is_err());
}

#[tokio::test]
async fn advanced_qec_codes_are_listed() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    let codes = client.advanced().qec_codes().await.expect("qec codes");
    let ids: Vec<&str> = codes.iter().map(|c| c.id.as_str()).collect();
    assert!(ids.contains(&"steane"));
    assert!(ids.contains(&"shor"));
    // The Steane code encodes 1 logical qubit into 7 physical qubits.
    let steane = codes.iter().find(|c| c.id == "steane").unwrap();
    assert_eq!(steane.n_physical, 7);
    assert_eq!(steane.n_logical, 1);
}

#[tokio::test]
async fn advanced_encode_and_syndrome_of_clean_state() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;
    let adv = client.advanced();

    let encoded = adv.encode("steane", Some(&[0])).await.expect("encode");
    assert_eq!(encoded.n_physical, 7);
    // A freshly encoded, error-free state has a trivial (all-zero) syndrome.
    let syn = adv.syndrome("steane", Some(&[0])).await.expect("syndrome");
    assert!(syn.syndrome.iter().all(|&s| s == 0));
}

#[tokio::test]
async fn advanced_quantum_kernel_matrix_is_valid() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    let data = vec![vec![0.1, 0.2], vec![0.9, 0.8], vec![0.15, 0.25]];
    let k = client
        .advanced()
        .kernel_matrix(&data, Some("zz"))
        .await
        .expect("kernel");
    assert_eq!(k.size, [3, 3]);
    // A kernel matrix has ones on the diagonal (a point is identical to itself).
    for i in 0..3 {
        assert!((k.matrix[i][i] - 1.0).abs() < 1e-6);
    }
}

#[tokio::test]
async fn advanced_ml_vqe_runs() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;
    use casq_sdk::advanced::{MlPauliTerm, VqeRunOptions};

    let hamiltonian = vec![MlPauliTerm::new("ZZ", 1.0), MlPauliTerm::new("XX", 0.5)];
    let r = client
        .advanced()
        .ml_vqe(
            &hamiltonian,
            "hardware_efficient",
            VqeRunOptions {
                max_iterations: Some(30),
                ..Default::default()
            },
        )
        .await
        .expect("ml vqe");
    assert!(!r.optimal_params.is_empty());
}

#[tokio::test]
async fn advanced_density_matrix_noise_simulation() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;
    use casq_sdk::advanced::{NoiseChannelConfig, NoiseSimOptions};

    // A Bell circuit.
    let mut bell = Circuit::new(2);
    bell.h(0).cx(0, 1);

    // Noiseless: pure state, fidelity 1.
    let clean = client
        .advanced()
        .simulate_noise(
            &bell,
            &[],
            NoiseSimOptions {
                compute_fidelity: true,
                ..Default::default()
            },
        )
        .await
        .expect("noiseless run");
    assert_eq!(clean.engine, "density-matrix");
    assert!(
        (clean.purity - 1.0).abs() < 1e-6,
        "expected pure state, got {}",
        clean.purity
    );
    assert!((clean.fidelity.unwrap() - 1.0).abs() < 1e-6);

    // Under depolarizing noise: mixed state, lower fidelity, error states appear.
    let noisy = client
        .advanced()
        .simulate_noise(
            &bell,
            &[NoiseChannelConfig::depolarizing(0.1)],
            NoiseSimOptions {
                compute_fidelity: true,
                shots: Some(2000),
                ..Default::default()
            },
        )
        .await
        .expect("noisy run");
    assert!(
        noisy.purity < 1.0,
        "noise should reduce purity, got {}",
        noisy.purity
    );
    assert!(noisy.fidelity.unwrap() < 1.0);
    let total: u64 = noisy.counts.values().sum();
    assert_eq!(total, 2000);

    // Amplitude damping on |1> relaxes toward |0>.
    let mut one = Circuit::new(1);
    one.x(0);
    let damped = client
        .advanced()
        .simulate_noise(
            &one,
            &[NoiseChannelConfig::amplitude_damping(0.5)],
            NoiseSimOptions::default(),
        )
        .await
        .expect("damped run");
    assert!((damped.probabilities.get("0").copied().unwrap_or(0.0) - 0.5).abs() < 1e-6);
}

#[tokio::test]
async fn backends_list_and_run() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;
    use casq_sdk::backends::BackendRunOptions;

    let backends = client.backends().list().await.expect("list backends");
    let ids: Vec<&str> = backends.iter().map(|b| b.id.as_str()).collect();
    assert!(ids.contains(&"local-simulator"));
    assert!(ids.contains(&"emulated-qpu"));
    // The remote QPU is registered but unavailable without credentials.
    let remote = backends.iter().find(|b| b.id == "remote-qpu").unwrap();
    assert!(!remote.available);
    assert!(!remote.capabilities.simulated);

    let mut bell = Circuit::new(2);
    bell.h(0).cx(0, 1);

    // Local simulator: exact, pure result.
    let local = client
        .backends()
        .run(
            "local-simulator",
            &bell,
            BackendRunOptions {
                shots: Some(1000),
                ..Default::default()
            },
        )
        .await
        .expect("run local");
    let total: u64 = local.counts.values().sum();
    assert_eq!(total, 1000);

    // Emulated QPU: baseline device noise degrades the state, and only half the
    // gates (CNOT, not H) are native.
    let emulated = client
        .backends()
        .run(
            "emulated-qpu",
            &bell,
            BackendRunOptions {
                shots: Some(1000),
                ..Default::default()
            },
        )
        .await
        .expect("run emulated");
    assert!(emulated.purity().unwrap() < 1.0);
    assert_eq!(emulated.native_gate_fraction(), Some(0.5));
}

#[tokio::test]
async fn jobs_submit_wait_and_target_a_backend() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;
    use casq_sdk::jobs::{JobStatus, SubmitJobOptions, WaitOptions};

    let mut bell = Circuit::new(2);
    bell.h(0).cx(0, 1);

    // Default (runner) path: submit -> queued -> wait -> completed with statevector.
    let submitted = client
        .jobs()
        .submit(
            &bell,
            SubmitJobOptions {
                shots: Some(1000),
                ..Default::default()
            },
        )
        .await
        .expect("submit");
    assert_eq!(submitted.status, JobStatus::Queued);

    let done = client
        .jobs()
        .wait_for(&submitted.id, WaitOptions::default())
        .await
        .expect("wait");
    assert_eq!(done.status, JobStatus::Completed);
    let result = done.result.expect("result");
    assert_eq!(result.results.counts.values().sum::<u64>(), 1000);
    assert!(!result.results.statevector.is_empty());

    // Backend-targeted: run on the emulated QPU — noisy, no statevector.
    let emulated = client
        .jobs()
        .submit(
            &bell,
            SubmitJobOptions {
                backend_id: Some("emulated-qpu".into()),
                shots: Some(1000),
                ..Default::default()
            },
        )
        .await
        .expect("submit emulated");
    let done = client
        .jobs()
        .wait_for(&emulated.id, WaitOptions::default())
        .await
        .expect("wait emulated");
    let result = done.result.expect("emulated result");
    assert_eq!(result.backend_id(), Some("emulated-qpu"));
    assert!(result.results.statevector.is_empty());

    // Clean up.
    client.jobs().delete(&submitted.id).await.expect("delete");
    client.jobs().delete(&emulated.id).await.expect("delete");
}

#[tokio::test]
async fn transpile_decomposes_to_native_basis() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    let mut bell = Circuit::new(2);
    bell.h(0).cx(0, 1);

    let result = client.transpile(&bell).await.expect("transpile");
    assert!(result.fully_native);
    assert!(result.unsupported.is_empty());
    // H decomposes to rotations, so the transpiled circuit is larger and native.
    assert!(result.transpiled_gate_count >= result.original_gate_count);
    for op in &result.operations {
        assert!(
            result.basis.contains(&op.gate),
            "non-native gate {}",
            op.gate
        );
    }

    // The transpiled circuit runs and measures like the original Bell state.
    let native = result.to_circuit(2);
    let run = client
        .run(&native, casq_sdk::RunOptions::new().shots(1000))
        .await
        .expect("run native");
    for state in run.counts().keys() {
        assert!(state == "00" || state == "11", "unexpected state {state}");
    }
}

#[tokio::test]
async fn transpile_routes_onto_linear_connectivity() {
    use casq_sdk::{Connectivity, TranspileOptions};
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    // cx(0,2) can't run on a line 0—1—2; routing must insert a SWAP.
    let mut circuit = Circuit::new(3);
    circuit.h(0).cx(0, 2);

    let result = client
        .transpile_with(
            &circuit,
            TranspileOptions::connectivity(Connectivity::Linear),
        )
        .await
        .expect("transpile+route");

    let swaps = result.swap_count.expect("routed result reports swapCount");
    assert!(swaps >= 1, "expected a SWAP for a non-adjacent CX");
    let perm = result
        .final_permutation
        .as_ref()
        .expect("routed result reports finalPermutation");
    assert_eq!(perm.len(), 3);

    // Every two-qubit gate now acts on adjacent physical qubits, all native.
    for op in &result.operations {
        assert!(result.basis.contains(&op.gate), "non-native {}", op.gate);
        if op.targets.len() == 2 {
            let d = op.targets[0].abs_diff(op.targets[1]);
            assert_eq!(d, 1, "two-qubit gate on non-adjacent {:?}", op.targets);
        }
    }
}

#[tokio::test]
async fn greedy_layout_cuts_swaps() {
    use casq_sdk::{Connectivity, Layout, TranspileOptions};
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    // cx(0,2) on a line: the identity layout needs a SWAP; a greedy layout can
    // seat the interacting qubits adjacent and avoid it entirely.
    let mut circuit = Circuit::new(3);
    circuit.h(0).cx(0, 2);

    let trivial = client
        .transpile_with(
            &circuit,
            TranspileOptions::connectivity(Connectivity::Linear).with_layout(Layout::Trivial),
        )
        .await
        .expect("trivial");
    let greedy = client
        .transpile_with(
            &circuit,
            TranspileOptions::connectivity(Connectivity::Linear).with_layout(Layout::Greedy),
        )
        .await
        .expect("greedy");

    assert_eq!(trivial.swap_count, Some(1));
    assert_eq!(greedy.swap_count, Some(0));
    assert!(greedy.initial_layout.is_some(), "greedy reports its layout");
}

#[tokio::test]
async fn multi_controlled_x_runs() {
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    // 3-control X: flip q3 iff q0,q1,q2 all set. Prepare the controls, then mcx.
    let mut circuit = Circuit::new(4);
    circuit.x(0).x(1).x(2).mcx(&[0, 1, 2], 3);

    let result = client
        .run(&circuit, RunOptions::new().shots(256))
        .await
        .expect("mcx run");
    // Deterministic: all four qubits end set.
    let counts = result.counts();
    assert_eq!(counts.keys().collect::<Vec<_>>(), vec!["1111"]);
}

#[tokio::test]
async fn sabre_router_inserts_no_more_swaps_than_greedy() {
    use casq_sdk::{Connectivity, Router, TranspileOptions};
    let Some(cfg) = config() else {
        return;
    };
    let client = authed_client(&cfg).await;

    // [cx(0,2), cx(0,1)] on a line: greedy needs 2 SWAPs, SABRE's lookahead 1.
    let mut circuit = Circuit::new(3);
    circuit.h(0).cx(0, 2).cx(0, 1);

    let greedy = client
        .transpile_with(
            &circuit,
            TranspileOptions::connectivity(Connectivity::Linear).with_router(Router::Greedy),
        )
        .await
        .expect("greedy");
    let sabre = client
        .transpile_with(
            &circuit,
            TranspileOptions::connectivity(Connectivity::Linear).with_router(Router::Sabre),
        )
        .await
        .expect("sabre");

    let (g, s) = (greedy.swap_count.unwrap(), sabre.swap_count.unwrap());
    assert!(s <= g, "SABRE ({s}) should not exceed greedy ({g})");
    assert_eq!((g, s), (2, 1));
}
