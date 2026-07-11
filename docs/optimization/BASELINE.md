# BASELINE — zai-rs 0.4.0 quality snapshot

This file records the observed quality baseline of the `e82be86` commit (0.4.0)
as captured during P00, against the fixed 1.88.0 toolchain and the pinned
nightly (`nightly-2026-07-10`, here resolved to the installed
`rustc 1.99.0-nightly (375b1431b 2026-07-10)`).

The raw command output, version strings and exit codes were captured to a
repo-external directory (`~/zai-rs-p00-baseline/`) per plan P00.1; this document
is the curated summary.

## Toolchain versions

| Tool | Version |
|---|---|
| rustc (1.88.0, MSRV) | `rustc 1.88.0 (6b00bc388 2025-06-23)` — host `aarch64-apple-darwin` |
| cargo (1.88.0) | `cargo 1.88.0 (873a06493 2025-05-10)` |
| rustc (nightly) | `rustc 1.99.0-nightly (375b1431b 2026-07-10)` |

`nightly-2026-07-10` is the pinned nightly alias; the installed default nightly
is built from the same commit.

## Repository metrics

| Metric | Value |
|---|---|
| Rust source lines (`src/**/*.rs`) | 27,777 |
| Cargo.lock package nodes | 316 |
| Public API symbols (rustdoc-json extracted) | 1,332 |
| `cargo package --list` files (pre-P00) | 227 |

## Quality gates (observed on 1.88.0)

| Gate | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | **pass** (exit 0) | formatting clean |
| `cargo clippy -p zai-rs --all-features --all-targets -- -D warnings` | **fail** (exit 1) | 59 `uninlined_format_args` lints in pre-existing 0.4 code — a clippy-version drift from the plan's stated baseline; all are style lints, not correctness. Fixed in P05's full migration. |
| `cargo test -p zai-rs --all-features --all-targets` | **pass** | 365 passed, 0 failed, 0 ignored (≈ 339 unit + 26 integration, matching plan §2.1) |
| `cargo test -p zai-rs --all-features --doc` | **pass** | 14 passed, 0 failed, 90 ignored (matching plan §2.1's ignored-doctest backlog) |
| `cargo audit --file Cargo.lock` | **fail** | `RUSTSEC-2026-0173` — `proc-macro-error2` unmaintained, introduced via `validator_derive 0.20.0`. Hotfixed in P01.9 (`validator_derive` pinned to 0.20.1). |
| `cargo package --list -p zai-rs` | **pass** | package builds (227 files pre-P00); the plan notes buildability ≠ protocol correctness |

## Divergences from the plan's stated baseline (§2.1)

The plan's §2.1 table reports clippy as passing on the baseline. Observed
reality on 1.88.0 stable: 59 `uninlined_format_args` style lints fire. These are
the result of a clippy lint that was promoted into `-D warnings` reach between
the plan's analysis and this run; they are style-only and do not indicate a
correctness regression. They are recorded here so the P05 migration (which
rewrites the affected files) clears them naturally.

All other baseline figures match the plan: 27,777 source lines, 365 passing
tests, 14 passing / 90 ignored doctests, and the `RUSTSEC-2026-0173` advisory.

## Coverage baseline

`cargo llvm-cov` was not re-run here (the plan §2.1 reports line 51.04%,
region 54.81%, function 44.34%); the exact numbers are re-established in P11
when the coverage gate is built. The P00 snapshot records the qualitative
finding: API-family coverage is severely imbalanced, with `agent`, `batches`,
`file`, `knowledge`, most model APIs, `realtime` and RMCP near 0%.

## Upstream contract baseline

All nine frozen upstream snapshots verified byte-exact against the plan's §3
fixed SHA-256 values (see `spec/upstream/SOURCES.toml`). The frozen OpenAPI
exposes exactly 59 operations across 53 paths, matching the plan and the
operation manifest in `spec/contracts/operations.json`.
