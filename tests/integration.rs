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
