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
