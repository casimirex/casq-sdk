//! Build and simulate a Bell state.
//!
//! Run against a live casimirQ server:
//! ```bash
//! CASQ_BASE_URL=http://localhost:8080/api/v1 \
//! CASQ_EMAIL=admin@example.com CASQ_PASSWORD=admin123 \
//!   cargo run --example bell_state
//! ```

use casq_sdk::{Circuit, Client, Engine, RunOptions};

#[tokio::main]
async fn main() -> casq_sdk::Result<()> {
    let base_url =
        std::env::var("CASQ_BASE_URL").unwrap_or_else(|_| "http://localhost:8080/api/v1".into());
    let email = std::env::var("CASQ_EMAIL").unwrap_or_else(|_| "admin@example.com".into());
    let password = std::env::var("CASQ_PASSWORD").unwrap_or_else(|_| "admin123".into());

    let mut client = Client::new(base_url)?;
    client.login(&email, &password).await?;

    let mut circuit = Circuit::new(2);
    circuit.h(0).cx(0, 1);

    let result = client
        .run(
            &circuit,
            RunOptions::new().engine(Engine::Statevector).shots(1024),
        )
        .await?;

    println!(
        "Bell state ({} shots on {})",
        result.shots, result.requested_engine
    );
    let mut counts: Vec<_> = result.counts().iter().collect();
    counts.sort_by_key(|(state, _)| (*state).clone());
    for (state, n) in counts {
        println!("  |{state}> : {n}");
    }
    println!(
        "execution time: {:.4} ms",
        result.metadata.execution_time_ms
    );
    Ok(())
}
