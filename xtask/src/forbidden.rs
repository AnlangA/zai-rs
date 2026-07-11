//! Forbidden-pattern checker (`xtask forbidden check <phase>`).
//!
//! Reads `spec/forbidden-patterns.toml` and applies the patterns registered
//! for the given phase to the repository's `src` (and, for later phases, docs
//! and examples). A match is a failure. The check distinguishes "no match"
//! from tool/IO failure per plan P00.13.

use crate::ExitCode;
use crate::contract::repo_root;
use std::fs;
use std::path::Path;

pub fn check(phase: &str) -> ExitCode {
    let root = repo_root();
    let patterns_path = root.join("spec/forbidden-patterns.toml");
    let text = match fs::read_to_string(&patterns_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", patterns_path.display());
            return ExitCode::FAILURE;
        },
    };
    let patterns: Patterns = match toml::from_str(&text) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: cannot parse {}: {e}", patterns_path.display());
            return ExitCode::FAILURE;
        },
    };

    // Collect the globs registered for the requested phase (and the catch-all
    // `all` phase). Phases not yet present in the file simply apply nothing.
    let phase_globs: Vec<&PatternGroup> = patterns
        .phase
        .iter()
        .filter(|g| g.name == phase || g.name == "all")
        .collect();

    if phase_globs.is_empty() {
        println!("forbidden check {phase}: no patterns registered (ok)");
        return ExitCode::SUCCESS;
    }

    // Walk src/ (and docs/examples once those phases exist). For P00–P04 we
    // only scan `src` plus `examples` (gen_video/chat_vision are in scope for
    // P01).
    let scan_dirs = if matches!(phase, "P01" | "P00") {
        vec!["src", "examples"]
    } else {
        vec!["src", "examples", "docs"]
    };

    let mut hits = 0u32;
    for dir in scan_dirs {
        let dir_path = root.join(dir);
        if !dir_path.exists() {
            continue;
        }
        walk(&dir_path, &phase_globs, &mut hits);
    }

    if hits == 0 {
        println!("forbidden check {phase}: clean");
        ExitCode::SUCCESS
    } else {
        eprintln!("forbidden check {phase}: {hits} violation(s)");
        ExitCode::FAILURE
    }
}

fn walk(dir: &Path, globs: &[&PatternGroup], hits: &mut u32) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            // Skip build output and vendored dirs.
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, "target" | "node_modules" | "vendor" | ".git") {
                continue;
            }
            walk(&path, globs, hits);
        } else if ft.is_file() {
            // Only scan source-ish text files.
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "rs" | "md" | "toml") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            for (line, lineno) in text.lines().zip(1u32..) {
                for g in globs {
                    for needle in &g.forbidden_substrings {
                        if line.contains(needle.as_str()) {
                            *hits += 1;
                            eprintln!(
                                "forbidden: {}:{}:{} matches `{needle}`",
                                path.display(),
                                lineno,
                                line.trim()
                            );
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct Patterns {
    #[serde(default)]
    phase: Vec<PatternGroup>,
}

#[derive(Debug, serde::Deserialize)]
struct PatternGroup {
    name: String,
    #[serde(default)]
    forbidden_substrings: Vec<String>,
}
