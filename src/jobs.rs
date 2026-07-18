//! The asynchronous job engine: submit a simulation, then poll it to completion.
//!
//! Obtain a handle with [`crate::Client::jobs`]. Submitting returns immediately
//! with a queued [`Job`]; the server runs it in the background (optionally on a
//! chosen backend). Poll with [`Jobs::get`], or block until it finishes with
//! [`Jobs::wait_for`].
//!
//! ```no_run
//! use casq_sdk::{Circuit, Client};
//! use casq_sdk::jobs::{SubmitJobOptions, WaitOptions};
//!
//! # async fn run() -> casq_sdk::Result<()> {
//! let mut client = Client::new("http://localhost:8080/api/v1")?;
//! client.login("admin@example.com", "admin123").await?;
//!
//! let mut bell = Circuit::new(2);
//! bell.h(0).cx(0, 1);
//!
//! let job = client.jobs().submit(&bell, SubmitJobOptions {
//!     backend_id: Some("emulated-qpu".into()),
//!     shots: Some(2000),
//!     ..Default::default()
//! }).await?;
//!
//! let done = client.jobs().wait_for(&job.id, WaitOptions::default()).await?;
//! if let Some(result) = done.result {
//!     println!("counts = {:?}", result.counts());
//! }
//! # Ok(())
//! # }
//! ```

use crate::advanced::NoiseChannelConfig;
use crate::circuit::Circuit;
use crate::client::Client;
use crate::error::{Error, Result};
use crate::simulation::{Engine, SimulationOutputs};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Lifecycle state of a job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Waiting to be processed.
    Queued,
    /// Currently executing.
    Running,
    /// Finished successfully.
    Completed,
    /// Finished with an error.
    Failed,
    /// Cancelled before it ran.
    Cancelled,
}

impl JobStatus {
    /// Whether this is a terminal (final) state.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
        )
    }
}

/// The result payload of a completed simulation job.
#[derive(Clone, Debug, Deserialize)]
pub struct SimulationJobResult {
    /// Number of qubits simulated.
    #[serde(rename = "numQubits")]
    pub num_qubits: usize,
    /// The engine or backend id the job ran on.
    #[serde(rename = "requestedEngine")]
    pub requested_engine: String,
    /// Shots sampled.
    pub shots: u32,
    /// Numerical outputs (statevector is empty for backend runs).
    pub results: SimulationOutputs,
    /// Timing / backend-specific metadata.
    pub metadata: Value,
}

impl SimulationJobResult {
    /// Sampled measurement counts.
    pub fn counts(&self) -> &HashMap<String, u64> {
        &self.results.counts
    }
    /// Execution time in milliseconds, if present.
    pub fn execution_time_ms(&self) -> Option<f64> {
        self.metadata.get("executionTimeMs").and_then(Value::as_f64)
    }
    /// The backend the job ran on, if it targeted one.
    pub fn backend_id(&self) -> Option<&str> {
        self.metadata.get("backendId").and_then(Value::as_str)
    }
}

/// A job record.
#[derive(Clone, Debug, Deserialize)]
pub struct Job {
    /// Server-assigned id.
    pub id: String,
    /// Job type, e.g. `"simulation"`.
    #[serde(rename = "type")]
    pub job_type: String,
    /// Lifecycle state.
    pub status: JobStatus,
    /// Progress in `[0, 1]`.
    pub progress: f64,
    /// The result, once completed.
    #[serde(default)]
    pub result: Option<SimulationJobResult>,
    /// The error, if the job failed.
    #[serde(default)]
    pub error: Option<String>,
    /// ISO-8601 timestamps.
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Last update time.
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    /// When processing started, if it has.
    #[serde(rename = "startedAt", default)]
    pub started_at: Option<String>,
    /// When it settled, if it has.
    #[serde(rename = "finishedAt", default)]
    pub finished_at: Option<String>,
}

/// A page of jobs.
#[derive(Clone, Debug, Deserialize)]
pub struct JobList {
    /// The jobs on this page, newest first.
    pub jobs: Vec<Job>,
    /// Total number of jobs owned by the user.
    pub total: usize,
}

/// Options for submitting a simulation job.
#[derive(Clone, Debug, Default)]
pub struct SubmitJobOptions {
    /// Human-readable name recorded with the run.
    pub circuit_name: Option<String>,
    /// Engine hint (ignored when a backend is chosen).
    pub engine: Option<Engine>,
    /// Backend to run on (see [`crate::Client::backends`]); default = the runner.
    pub backend_id: Option<String>,
    /// Noise channels, for noise-capable backends.
    pub noise: Vec<NoiseChannelConfig>,
    /// Measurement shots.
    pub shots: Option<u32>,
    /// Seed for reproducible sampling.
    pub seed: Option<u32>,
}

/// Options for [`Jobs::wait_for`].
#[derive(Clone, Copy, Debug)]
pub struct WaitOptions {
    /// Delay between status polls.
    pub poll_interval: Duration,
    /// Give up after this long.
    pub timeout: Duration,
}

impl Default for WaitOptions {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(300),
            timeout: Duration::from_secs(60),
        }
    }
}

/// Handle for the async job engine, borrowed from a [`Client`].
pub struct Jobs<'a> {
    pub(crate) client: &'a Client,
}

impl Jobs<'_> {
    /// Submit a simulation job. Returns immediately with a queued [`Job`].
    pub async fn submit(&self, circuit: &Circuit, options: SubmitJobOptions) -> Result<Job> {
        let mut body = serde_json::json!({
            "numQubits": circuit.num_qubits(),
            "operations": circuit.operations(),
        });
        if let Some(name) = options.circuit_name {
            body["circuitName"] = serde_json::json!(name);
        }
        if let Some(engine) = options.engine {
            body["engine"] = serde_json::to_value(engine)?;
        }
        if let Some(backend) = options.backend_id {
            body["backendId"] = serde_json::json!(backend);
        }
        if !options.noise.is_empty() {
            body["noise"] = serde_json::json!(options.noise);
        }
        if let Some(shots) = options.shots {
            body["shots"] = serde_json::json!(shots);
        }
        if let Some(seed) = options.seed {
            body["seed"] = serde_json::json!(seed);
        }
        self.client.post("/jobs", &body).await
    }

    /// Fetch a job's current state.
    pub async fn get(&self, id: &str) -> Result<Job> {
        self.client.get(&format!("/jobs/{id}")).await
    }

    /// List the user's jobs, newest first.
    pub async fn list(&self, page: usize, limit: usize) -> Result<JobList> {
        self.client
            .get(&format!("/jobs?page={page}&limit={limit}"))
            .await
    }

    /// Cancel a still-queued job.
    pub async fn cancel(&self, id: &str) -> Result<Job> {
        self.client
            .post(&format!("/jobs/{id}/cancel"), &serde_json::json!({}))
            .await
    }

    /// Delete a job.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.client.delete(&format!("/jobs/{id}")).await
    }

    /// Poll a job until it reaches a terminal state, or the timeout elapses.
    pub async fn wait_for(&self, id: &str, options: WaitOptions) -> Result<Job> {
        let deadline = std::time::Instant::now() + options.timeout;
        loop {
            let job = self.get(id).await?;
            if job.status.is_terminal() {
                return Ok(job);
            }
            if std::time::Instant::now() >= deadline {
                return Err(Error::Timeout {
                    after_ms: options.timeout.as_millis() as u64,
                });
            }
            tokio::time::sleep(options.poll_interval).await;
        }
    }
}
