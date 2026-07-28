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
controlled rotations `crx cry crz`, `ccx`/`toffoli cswap`, multi-controlled
`mcx mcz ccz`, `measure`/`measure_all`.
Use `Circuit::push(Operation)` for anything else.

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
| `shor(number)` | `ShorResult` (genuine QPE order finding) |
| `teleport(alpha, beta)` | `TeleportResult` |
| `vqe(n, hamiltonian, max_iterations)` | `VqeResult` |
| `qaoa(n, edges, p)` | `QaoaResult` |
| `deutsch_jozsa(n, oracle, value, mask)` | `DeutschJozsaResult` |
| `bernstein_vazirani(n, secret)` | `BernsteinVaziraniResult` |
| `simon(n, secret)` | `SimonResult` |
| `phase_estimation(phi, precision)` | `PhaseEstimationResult` |
| `amplitude_amplification(angles, good_states, iterations)` | `AmplitudeAmplificationResult` |
| `quantum_walk(n, steps, start, symmetric_coin)` | `QuantumWalkResult` |
| `hamiltonian_simulation(n, terms, time, steps, order, initial_ones)` | `HamiltonianSimulationResult` |
| `hhl(b0, b1)` | `HhlResult` |
| `vqe_examples()` / `qaoa_examples()` | example inputs |

## Advanced features

Beyond the textbook algorithms, `client.advanced()` exposes error correction,
noise modeling, and quantum machine learning:

```rust
let adv = client.advanced();

// Quantum error correction (Steane, Shor codes)
let codes = adv.qec_codes().await?;                  // properties of each code
let encoded = adv.encode("steane", Some(&[0])).await?;
let syndrome = adv.syndrome("steane", Some(&[0])).await?;

// Noise modeling
let catalog = adv.noise_catalog().await?;            // channels + device models
let dev = adv.characterize("ibmq_lagos").await?;     // T1/T2, error rates, ...

// Quantum machine learning
use casq_sdk::advanced::{MlPauliTerm, VqeRunOptions};
let ml = adv.ml_catalog().await?;                    // ansatze + feature maps
let vqe = adv.ml_vqe(
    &[MlPauliTerm::new("ZZ", 1.0), MlPauliTerm::new("XX", 0.5)],
    "hardware_efficient",
    VqeRunOptions { max_iterations: Some(50), ..Default::default() },
).await?;
let kernel = adv.kernel_matrix(&data, Some("zz")).await?; // quantum kernel (QSVM)

// Density-matrix noise simulation
use casq_sdk::{Circuit, advanced::{NoiseChannelConfig, NoiseSimOptions}};
let mut bell = Circuit::new(2);
bell.h(0).cx(0, 1);
let result = adv.simulate_noise(
    &bell,
    &[NoiseChannelConfig::depolarizing(0.1)],
    NoiseSimOptions { compute_fidelity: true, shots: Some(2000), ..Default::default() },
).await?;
println!("purity {:.3}, fidelity {:.3}", result.purity, result.fidelity.unwrap());
```

| Method | Returns |
| --- | --- |
| `qec_codes()` | `Vec<QecCode>` |
| `encode(code, logical_state)` | `EncodedState` |
| `syndrome(code, logical_state)` | `SyndromeResult` |
| `noise_catalog()` | `NoiseCatalog` |
| `validate_noise(channels)` | `NoiseValidation` |
| `characterize(model)` | `DeviceCharacteristics` |
| `ml_catalog()` | `MlCatalog` |
| `ml_vqe(hamiltonian, ansatz, opts)` | `MlVqeResult` |
| `kernel_matrix(data, feature_map)` | `KernelMatrix` |
| `simulate_noise(circuit, noise, opts)` | `NoiseSimulationResult` (density-matrix engine) |

## Backends

`client.backends()` lists the execution targets (simulators, an emulated device,
and — when configured — a real QPU) and runs a circuit on a chosen one. Selecting
where a circuit runs is just a backend id.

```rust
use casq_sdk::backends::BackendRunOptions;

let backends = client.backends().list().await?;   // id, type, availability, capabilities

let mut bell = Circuit::new(2);
bell.h(0).cx(0, 1);

let result = client.backends()
    .run("emulated-qpu", &bell, BackendRunOptions { shots: Some(2000), ..Default::default() })
    .await?;
println!("purity {:?}, native fraction {:?}",
    result.purity(), result.native_gate_fraction());
```

| Method | Returns |
| --- | --- |
| `list()` | `Vec<Backend>` |
| `get(id)` | `Backend` |
| `run(id, circuit, opts)` | `BackendRunResult` |

## Transpilation

`client.transpile()` decomposes a circuit into the native gate basis a real
device would run (`rz`, `ry`, `cx`) — non-native gates are rewritten, and the
result reports the gate-count cost.

```rust
let mut bell = Circuit::new(2);
bell.h(0).cx(0, 1);

let t = client.transpile(&bell).await?;
println!("{} -> {} gates, fully native: {}",
    t.original_gate_count, t.transpiled_gate_count, t.fully_native);

// Run the native circuit.
let native = t.to_circuit(2);
let result = client.run(&native, RunOptions::new().shots(1000)).await?;
```

### Routing onto hardware connectivity

Real devices only run a two-qubit gate between *coupled* qubits.
`transpile_with` routes a circuit onto a connectivity, inserting SWAPs so every
two-qubit gate acts on adjacent qubits. The result reports `swap_count` and a
`final_permutation` — the logical→physical layout after routing (read logical
qubit `l` from physical `final_permutation[l]`).

```rust
use casq_sdk::{Connectivity, TranspileOptions};

let mut c = Circuit::new(3);
c.h(0).cx(0, 2);   // 0 and 2 aren't adjacent on a line

let t = client
    .transpile_with(&c, TranspileOptions::connectivity(Connectivity::Linear))
    .await?;
println!("inserted {} SWAP(s); layout = {:?}",
    t.swap_count.unwrap(), t.final_permutation.unwrap());

// Or route onto an explicit (non-linear) coupling map:
let star = TranspileOptions::coupling(vec![[0, 1], [0, 2], [0, 3]]);
let t = client.transpile_with(&c, star).await?;
```

A **smarter initial layout** can cut the SWAP count: instead of starting from
the identity placement, `Layout::Greedy` seats interacting qubits near each
other. The result reports the chosen `initial_layout` (where each logical qubit
starts) alongside `final_permutation` and `swap_count`.

```rust
use casq_sdk::{Connectivity, Layout, TranspileOptions};

let opts = TranspileOptions::connectivity(Connectivity::Linear).with_layout(Layout::Greedy);
let t = client.transpile_with(&c, opts).await?;
// For cx(0,2) on a line this drops from 1 SWAP to 0: the two interacting
// qubits are placed on adjacent wires from the start.
println!("initial layout = {:?}, swaps = {}",
    t.initial_layout.unwrap(), t.swap_count.unwrap());
```

The **router** — how SWAPs are inserted once routing is unavoidable — can also
be chosen. `Router::Sabre` looks ahead over upcoming gates and usually inserts
fewer SWAPs than the default per-gate greedy router.

```rust
use casq_sdk::{Connectivity, Router, TranspileOptions};

let opts = TranspileOptions::connectivity(Connectivity::Linear).with_router(Router::Sabre);
let t = client.transpile_with(&c, opts).await?;
```

## Async jobs

`client.jobs()` submits simulations to the background job engine (optionally on a
chosen backend) and polls them. Submit returns immediately; `wait_for` blocks
until the job settles.

```rust
use casq_sdk::jobs::{SubmitJobOptions, WaitOptions};

let job = client.jobs().submit(&bell, SubmitJobOptions {
    backend_id: Some("emulated-qpu".into()),   // run on any backend
    shots: Some(2000),
    ..Default::default()
}).await?;

let done = client.jobs().wait_for(&job.id, WaitOptions::default()).await?;
if let Some(result) = done.result {
    println!("counts = {:?}", result.counts());
}
```

| Method | Returns |
| --- | --- |
| `submit(circuit, opts)` | `Job` (queued) |
| `get(id)` / `list(page, limit)` | `Job` / `JobList` |
| `wait_for(id, opts)` | `Job` (terminal) |
| `cancel(id)` / `delete(id)` | `Job` / `()` |

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

### Contract test

`cargo test` also runs an offline **contract test** (`tests/contract.rs`) that
validates every endpoint and request field the SDK uses against a vendored copy
of the platform's OpenAPI spec (`tests/openapi.json`) — so CI fails the moment
this reference client drifts from the API contract. Refresh the vendored spec
after an API change:

```bash
scripts/refresh-openapi.sh ../casimirQ/openapi.json
```

The contract test only checks the SDK against that *vendored* copy, which can go
stale. CI's **`openapi-sync`** job closes that gap: it checks out casimirQ's
canonical `openapi.json` and diffs it against `tests/openapi.json`
(`scripts/check-openapi-sync.sh`), failing when an upstream DTO change hasn't
been vendored here. It reads the private platform repo through a repo secret —
add **`CASIMIRQ_TOKEN`** (a fine-grained PAT or App token with read access to
`casimirex/casimirQ`) under *Settings → Secrets and variables → Actions*.

## Related

- [casimirQ](../casimirQ) — the quantum simulation platform this client targets.
- [casq-tutorial](../casq-tutorial) — a novice-to-professional course built on this SDK.
- [Ecosystem roadmap](../casimirQ/ROADMAP.md) — where the platform, SDK, and tutorial are headed.

## License

MIT
