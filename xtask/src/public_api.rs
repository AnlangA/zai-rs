//! Public-API surface tracker (`xtask public-api check`).
//!
//! Plan P00.9 establishes the baseline surface in spec/contracts/public-api.toml:
//! every 0.4 symbol identity is marked `removed`, every 0.5 symbol identity is
//! marked `added`. xtask `public-api check` reads the committed public-api-0.4.json
//! snapshot and confirms it is fully classified in public-api.toml. The actual
//! "no unclassified symbol" gate against the live crate is implemented in P10,
//! once the 0.5 surface exists.

use crate::ExitCode;
use crate::contract::repo_root;
use std::fs;

pub fn check() -> ExitCode {
    let root = repo_root();
    let snapshot_path = root.join("spec/contracts/public-api-0.4.json");
    let toml_path = root.join("spec/contracts/public-api.toml");

    if !snapshot_path.exists() {
        eprintln!("error: {} missing", snapshot_path.display());
        return ExitCode::FAILURE;
    }
    if !toml_path.exists() {
        eprintln!("error: {} missing", toml_path.display());
        return ExitCode::FAILURE;
    }

    // Confirm every path in the baseline snapshot is classified (removed) in
    // the toml. The live 0.5 surface gate is added in P10.
    let snapshot_text = match fs::read_to_string(&snapshot_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: read {}: {e}", snapshot_path.display());
            return ExitCode::FAILURE;
        },
    };
    let toml_text = match fs::read_to_string(&toml_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: read {}: {e}", toml_path.display());
            return ExitCode::FAILURE;
        },
    };

    #[derive(serde::Deserialize)]
    struct ApiSnapshot {
        path: String,
    }
    let snapshots: Vec<ApiSnapshot> = match serde_json::from_str(&snapshot_text) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: parse {}: {e}", snapshot_path.display());
            return ExitCode::FAILURE;
        },
    };

    #[derive(serde::Deserialize)]
    struct ApiToml {
        symbol: Vec<ApiSymbol>,
    }
    #[derive(serde::Deserialize)]
    struct ApiSymbol {
        path: String,
        #[allow(dead_code)]
        status: String,
    }
    let api: ApiToml = match toml::from_str(&toml_text) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: parse {}: {e}", toml_path.display());
            return ExitCode::FAILURE;
        },
    };

    let classified: std::collections::HashSet<&str> =
        api.symbol.iter().map(|s| s.path.as_str()).collect();
    let mut unclassified = 0u32;
    for snap in &snapshots {
        if !classified.contains(snap.path.as_str()) {
            unclassified += 1;
            eprintln!("unclassified baseline symbol: {}", snap.path);
        }
    }

    if unclassified == 0 {
        println!(
            "public-api: all {} baseline symbols classified",
            snapshots.len()
        );
        // Note: the live 0.5 gate (no unclassified current symbol) is enforced
        // from P10 onward, once the 0.5 surface exists.
        ExitCode::SUCCESS
    } else {
        eprintln!("public-api: {unclassified} unclassified baseline symbol(s)");
        ExitCode::FAILURE
    }
}
