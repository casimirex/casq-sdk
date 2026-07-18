//! # casq-sdk
//!
//! An async Rust client for the **casimirQ** quantum circuit simulation platform.
//!
//! Build circuits with a fluent gate API, run them on real simulation engines
//! (statevector, Clifford, MPS), and call the pre-built quantum algorithms —
//! all against a running casimirQ server over its REST API.
//!
//! ## Quick start
//!
//! ```no_run
//! use casq_sdk::{Circuit, Client, RunOptions, Engine};
//!
//! #[tokio::main]
//! async fn main() -> casq_sdk::Result<()> {
//!     // Point at your API (the URL before resource paths).
//!     let mut client = Client::new("http://localhost:8080/api/v1")?;
//!     client.login("admin@example.com", "admin123").await?;
//!
//!     // Build a Bell state.
//!     let mut circuit = Circuit::new(2);
//!     circuit.h(0).cx(0, 1);
//!
//!     // Run it.
//!     let result = client
//!         .run(&circuit, RunOptions::new().engine(Engine::Statevector).shots(1024))
//!         .await?;
//!     println!("counts = {:?}", result.counts());
//!
//!     // Or call a pre-built algorithm.
//!     let grover = client.algorithms().grover(4, 9, None).await?;
//!     println!("Grover success probability = {:.4}", grover.success_probability);
//!     Ok(())
//! }
//! ```
//!
//! ## Modules
//!
//! - [`Client`] — authentication, circuit persistence, simulation, algorithms.
//! - [`Circuit`] — local circuit construction with a Qiskit-style gate API.
//! - [`algorithms`] — typed wrappers for QFT, Grover, Shor, VQE, QAOA, teleport.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod advanced;
pub mod algorithms;
mod circuit;
mod client;
mod error;
mod models;
mod simulation;

pub use circuit::{Circuit, Operation};
pub use client::Client;
pub use error::{Error, Result};
pub use models::{AuthToken, CircuitList, CircuitRecord, CircuitSummary, Pagination, User};
pub use simulation::{
    Amplitude, Engine, RunOptions, SimulationMetadata, SimulationOutputs, SimulationResult,
};
