# casq-sdk

An async **Rust** client for the [casimirQ](https://github.com/) quantum circuit
simulation platform. Build circuits with a fluent gate API, run them on real
simulation engines (statevector, Clifford, MPS), persist them per user, and call
the pre-built quantum algorithms — all over casimirQ's REST API.

It mirrors the ergonomics of higher-level quantum SDKs (`circuit.h(0); circuit.cx(0,1); run(...)`)
while staying idiomatic Rust: `async`/`await`, `Result`-based errors, and typed
responses.

## Install

```toml
[dependencies]
casq-sdk = { path = "../casq-sdk" }   # or a git/registry source
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick start

```rust
use casq_sdk::{Circuit, Client, Engine, RunOptions};

#[tokio::main]
async fn main() -> casq_sdk::Result<()> {
    // The API base URL (the part before resource paths).
    let mut client = Client::new("http://localhost:8080/api/v1")?;
    client.login("admin@example.com", "admin123").await?;

    // Build a Bell state.
    let mut circuit = Circuit::new(2);
    circuit.h(0).cx(0, 1);

    // Run it (stateless — nothing is persisted).
    let result = client
        .run(&circuit, RunOptions::new().engine(Engine::Statevector).shots(1024))
        .await?;

    println!("counts = {:?}", result.counts());          // {"00": 512, "11": 512}
    println!("most probable = {:?}", result.most_probable());
    Ok(())
}
```

## Building circuits

`Circuit` offers a Qiskit-style, chainable gate API. Controls are folded into the
target list (e.g. `cx` → `[control, target]`); rotations carry their angle.

```rust
let mut c = Circuit::new(3);
c.h(0)
 .cx(0, 1)
 .ccx(0, 1, 2)          // Toffoli (alias: `toffoli`)
 .rx(2, std::f64::consts::FRAC_PI_2)
 .measure_all();
```

Available gates: `h x y z s sdg t tdg`, `rx ry rz p`, `cx`/`cnot cy cz ch swap cp`,
`ccx`/`toffoli cswap`, `measure`/`measure_all`. Use `Circuit::push(Operation)` for
anything else.

## Persisting circuits

```rust
let record = client.create_circuit("Bell", &circuit).await?;
let page   = client.list_circuits(1, 20).await?;   // { circuits, pagination }
let same   = client.get_circuit(&record.id).await?;
let sim    = client.run_stored(&record.id, RunOptions::new().shots(512)).await?;
client.delete_circuit(&record.id).await?;
```

## Algorithms

Typed wrappers for every pre-built algorithm, via `client.algorithms()`:

```rust
let algos = client.algorithms();

let grover = algos.grover(4, 9, None).await?;      // marked item 9, optimal iterations
println!("success = {:.4}", grover.success_probability);

let shor = algos.shor(15).await?;
println!("factors = {:?}", shor.factors);          // [3, 5]

let qft  = algos.qft(3).await?;
let tele = algos.teleport(0.6, 0.8).await?;

// VQE / QAOA can be seeded from built-in examples:
let examples = algos.vqe_examples().await?;
let h2 = &examples["H2"];
let n  = h2.iter().flat_map(|t| t.qubits.iter().copied()).max().map_or(1, |m| m + 1);
let vqe = algos.vqe(n, h2, Some(100)).await?;
println!("ground-state energy ≈ {:.4}", vqe.optimal_energy);
```

| Method | Returns |
| --- | --- |
| `list()` | `Vec<AlgorithmInfo>` |
| `qft(n)` | `QftResult` |
| `grover(n, marked, iterations)` | `GroverResult` |
| `shor(number)` | `ShorResult` |
| `teleport(alpha, beta)` | `TeleportResult` |
| `vqe(n, hamiltonian, max_iterations)` | `VqeResult` |
| `qaoa(n, edges, p)` | `QaoaResult` |
| `vqe_examples()` / `qaoa_examples()` | example inputs |

## Authentication

`login`/`signup` store the returned JWT on the client for subsequent calls. You
can also inject a token directly:

```rust
let client = Client::with_token("http://localhost:8080/api/v1", token)?;
```

Calls that require auth fail fast with `Error::NotAuthenticated` if no token is set.

## Errors

Everything returns `casq_sdk::Result<T>`. `Error` distinguishes transport failures
(`Transport`), server rejections (`Api { status, message }` — the message is pulled
from casimirQ's error envelope), missing auth (`NotAuthenticated`), and
(de)serialization issues (`Serde`).

## Examples

```bash
CASQ_BASE_URL=http://localhost:8080/api/v1 \
CASQ_EMAIL=admin@example.com CASQ_PASSWORD=admin123 \
  cargo run --example bell_state      # also: algorithms, grover
```

## Testing

```bash
cargo test                            # offline unit + doc tests (no server needed)

# end-to-end tests against a running casimirQ server:
CASQ_BASE_URL=http://localhost:8080/api/v1 \
CASQ_EMAIL=admin@example.com CASQ_PASSWORD=admin123 \
  cargo test --test integration -- --nocapture
```

The integration tests are skipped automatically when `CASQ_BASE_URL` is unset.

## License

MIT
