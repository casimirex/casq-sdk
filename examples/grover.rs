//! Run Grover's search via the algorithms API.
//!
//! ```bash
//! CASQ_BASE_URL=http://localhost:8080/api/v1 \
//! CASQ_EMAIL=admin@example.com CASQ_PASSWORD=admin123 \
//!   cargo run --example grover
//! ```

use casq_sdk::Client;

#[tokio::main]
async fn main() -> casq_sdk::Result<()> {
    let base_url =
        std::env::var("CASQ_BASE_URL").unwrap_or_else(|_| "http://localhost:8080/api/v1".into());
    let email = std::env::var("CASQ_EMAIL").unwrap_or_else(|_| "admin@example.com".into());
    let password = std::env::var("CASQ_PASSWORD").unwrap_or_else(|_| "admin123".into());

    let mut client = Client::new(base_url)?;
    client.login(&email, &password).await?;

    for n in 2..=5 {
        let marked = (1usize << n) - 2; // an arbitrary marked item < 2^n
        let g = client.algorithms().grover(n, marked, None).await?;
        println!(
            "n={n} marked={marked}: success={:.4} optimal_iterations={}",
            g.success_probability, g.optimal_iterations
        );
    }
    Ok(())
}
