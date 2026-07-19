//! Contract test: validate this SDK against the casimirQ OpenAPI spec.
//!
//! `tests/openapi.json` is a vendored copy of the platform's contract (refresh
//! it with `scripts/refresh-openapi.sh`). This test asserts that:
//!
//!   1. every endpoint the SDK calls exists in the spec (method + path), and
//!   2. every request field the SDK sends is defined on the matching DTO schema.
//!
//! It is fully offline (no server needed) and runs as part of `cargo test`, so
//! CI fails the moment the Rust reference client drifts from the API contract.

use serde_json::Value;

fn spec() -> Value {
    let raw = include_str!("openapi.json");
    serde_json::from_str(raw).expect("vendored openapi.json is valid JSON")
}

/// (method, path) for every endpoint the SDK issues. Path params use the spec's
/// template form (e.g. `{id}`), since the SDK substitutes concrete values.
const ENDPOINTS: &[(&str, &str)] = &[
    // auth (client.rs)
    ("post", "/api/v1/auth/login"),
    ("post", "/api/v1/auth/signup"),
    // circuits (client.rs)
    ("post", "/api/v1/circuits"),
    ("get", "/api/v1/circuits"),
    ("get", "/api/v1/circuits/{id}"),
    ("delete", "/api/v1/circuits/{id}"),
    ("post", "/api/v1/circuits/{id}/simulate"),
    // algorithms (algorithms.rs)
    ("get", "/api/v1/algorithms"),
    ("post", "/api/v1/algorithms/qft"),
    ("post", "/api/v1/algorithms/grover"),
    ("post", "/api/v1/algorithms/shor"),
    ("post", "/api/v1/algorithms/teleport"),
    ("post", "/api/v1/algorithms/vqe"),
    ("post", "/api/v1/algorithms/qaoa"),
    ("get", "/api/v1/algorithms/vqe/examples"),
    ("get", "/api/v1/algorithms/qaoa/examples"),
    // advanced (advanced.rs)
    ("get", "/api/v1/advanced/error-correction/codes"),
    ("post", "/api/v1/advanced/error-correction/{codeId}/encode"),
    ("post", "/api/v1/advanced/error-correction/syndrome"),
    ("get", "/api/v1/advanced/noise/channels"),
    ("post", "/api/v1/advanced/noise/apply"),
    ("post", "/api/v1/advanced/noise/characterize"),
    ("get", "/api/v1/advanced/ml/vqe/ansatz"),
    ("post", "/api/v1/advanced/ml/vqe/run"),
    ("post", "/api/v1/advanced/ml/kernel/matrix"),
    ("post", "/api/v1/advanced/noise/simulate"),
    // backends (backends.rs)
    ("get", "/api/v1/backends"),
    ("get", "/api/v1/backends/{id}"),
    ("post", "/api/v1/backends/{id}/run"),
    // jobs (jobs.rs)
    ("post", "/api/v1/jobs"),
    ("get", "/api/v1/jobs"),
    ("get", "/api/v1/jobs/{id}"),
    ("post", "/api/v1/jobs/{id}/cancel"),
    ("delete", "/api/v1/jobs/{id}"),
    // transpiler (client.transpile)
    ("post", "/api/v1/transpile"),
];

#[test]
fn every_sdk_endpoint_exists_in_the_spec() {
    let spec = spec();
    let paths = &spec["paths"];

    let mut missing = Vec::new();
    for (method, path) in ENDPOINTS {
        let op = paths.get(path).and_then(|p| p.get(method));
        if op.is_none() {
            missing.push(format!("{} {}", method.to_uppercase(), path));
        }
    }

    assert!(
        missing.is_empty(),
        "the SDK calls endpoints not in the OpenAPI contract:\n  {}\n\
         (refresh tests/openapi.json, or fix the SDK / server)",
        missing.join("\n  "),
    );
}

/// For a DTO-backed request, the fields the SDK sends must exist on the schema.
fn schema_props<'a>(spec: &'a Value, dto: &str) -> Vec<&'a str> {
    spec["components"]["schemas"][dto]["properties"]
        .as_object()
        .unwrap_or_else(|| panic!("schema {dto} has no properties"))
        .keys()
        .map(String::as_str)
        .collect()
}

#[test]
fn sdk_request_fields_match_dto_schemas() {
    let spec = spec();

    // (DTO schema name, fields the SDK sends in its request body).
    let checks: &[(&str, &[&str])] = &[
        ("QFTDto", &["n"]),
        ("GroverDto", &["n", "markedItem", "iterations"]),
        ("ShorDto", &["N"]),
        ("TeleportDto", &["alpha", "beta"]),
        ("VQEDto", &["n", "hamiltonian", "maxIterations"]),
        ("QAOADto", &["n", "edges", "p"]),
        (
            "SimulateNoiseDto",
            &[
                "numQubits",
                "operations",
                "noise",
                "shots",
                "seed",
                "computeFidelity",
            ],
        ),
        (
            "RunOnBackendDto",
            &["numQubits", "operations", "shots", "seed", "noise"],
        ),
        (
            "TranspileDto",
            &[
                "numQubits",
                "operations",
                "connectivity",
                "coupling",
                "layout",
                "router",
            ],
        ),
        (
            "SubmitSimulationJobDto",
            &[
                "circuitName",
                "numQubits",
                "operations",
                "engine",
                "backendId",
                "noise",
                "shots",
                "seed",
            ],
        ),
    ];

    let mut problems = Vec::new();
    for (dto, fields) in checks {
        let props = schema_props(&spec, dto);
        for field in *fields {
            if !props.contains(field) {
                problems.push(format!(
                    "{dto} is missing SDK field `{field}` (has {props:?})"
                ));
            }
        }
    }

    assert!(
        problems.is_empty(),
        "request-field drift:\n  {}",
        problems.join("\n  ")
    );
}
