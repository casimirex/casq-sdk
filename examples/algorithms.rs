//! Tour of every pre-built algorithm.
//!
//! ```bash
//! CASQ_BASE_URL=http://localhost:8080/api/v1 \
//! CASQ_EMAIL=admin@example.com CASQ_PASSWORD=admin123 \
//!   cargo run --example algorithms
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
    let algos = client.algorithms();

    println!("Available algorithms:");
    for info in algos.list().await? {
        println!("  - {} [{}]", info.name, info.category);
    }
    println!();

    let qft = algos.qft(3).await?;
    println!(
        "QFT(3):     qubits={} gates={} depth={}",
        qft.qubits, qft.gate_count, qft.depth
    );

    let grover = algos.grover(4, 9, None).await?;
    println!("Grover:     success={:.4}", grover.success_probability);

    let shor = algos.shor(15).await?;
    println!("Shor(15):   factors={:?}", shor.factors);

    let tele = algos.teleport(0.6, 0.8).await?;
    println!(
        "Teleport:   fidelity={:.4} verified={}",
        tele.fidelity, tele.verified
    );

    // VQE seeded from a built-in example Hamiltonian.
    let vqe_examples = algos.vqe_examples().await?;
    if let Some(h2) = vqe_examples.get("H2") {
        let n = h2
            .iter()
            .flat_map(|t| t.qubits.iter().copied())
            .max()
            .map_or(1, |m| m + 1);
        let vqe = algos.vqe(n, h2, Some(100)).await?;
        println!(
            "VQE(H2):    energy={:.4} converged={}",
            vqe.optimal_energy, vqe.converged
        );
    }

    // QAOA seeded from a built-in example graph.
    let qaoa_examples = algos.qaoa_examples().await?;
    if let Some(triangle) = qaoa_examples.get("triangle") {
        let qaoa = algos.qaoa(triangle.n, &triangle.edges, Some(1)).await?;
        println!(
            "QAOA(tri):  best_cut={} expectation={:.4}",
            qaoa.best_cut_value, qaoa.max_expectation
        );
    }

    Ok(())
}
