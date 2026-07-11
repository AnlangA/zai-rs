# EXECUTION_LEDGER — zai-rs 0.5.0 optimization plan

One row per task (P00–P15), in execution order. Each row records the task ID,
its fixed commit title, the parent commit, the verification commands run and
their outcome, plus any residual issues (which may only reference an
already-scheduled task ID).

Commit hashes are derived from `git log --format='%H%x09%s'` after each task;
this file records parent commits and results, not the task's own hash.

| Task | Commit title | Parent | Verification | Result | Residual |
|---|---|---|---|---|---|
| P00 | `test(contract): freeze 2026-07-11 upstream specifications` | `175cd04` | `xtask contract verify`, `xtask contract check`, `cargo test -p xtask`, `git diff --check` | see P00 notes below | — |

## P00 — freeze upstream specifications and reproducible baseline

- **Status:** complete
- **Commit title (fixed):** `test(contract): freeze 2026-07-11 upstream specifications`
- **Parent commit:** `175cd04 docs: add 0.5.0 AI agent optimization execution plan`

### Deliverables

1. 9 upstream contract snapshots frozen under `spec/upstream/`, all byte-exact
   against plan §3 fixed SHA-256 values (OpenAPI 59 ops / 53 paths, AsyncAPI,
   5 manual pages, coding-plan source). Provenance in `spec/upstream/SOURCES.toml`.
2. `xtask` workspace crate with working `contract {verify,generate,check}`,
   `forbidden check <phase>`, `public-api check`; later-task commands stubbed
   with exit-code-2 "not yet" markers.
3. `spec/contracts/operations.json` — 59 operations, stable-sorted,
   idempotent regeneration (two consecutive `generate` runs produce no diff).
4. `spec/contracts/manual-constraints.toml` — plan §13 encoded as TOML.
5. `spec/contracts/coverage.toml` — 59 OpenAPI ops + 1 Coding Plan + 5 Realtime
   paths, all `status = "missing"` (flipped to `covered` in P06).
6. `spec/contracts/public-api-0.4.json` + `public-api.toml` — 0.4 baseline
   surface (1332 symbols) marked `removed`, 0.5 target surface marked `added`.
7. `rust-toolchain.toml` — default channel 1.88.0, components pinned.
8. `scripts/bootstrap-tools.sh` — pinned tool versions (P11/P14 tools).
9. `spec/package-allowlist.txt`, `spec/forbidden-patterns.toml`.
10. `docs/optimization/BASELINE.md` + baseline capture at `~/zai-rs-p00-baseline/`.

### Verification commands and results

| Command | Exit | Notes |
|---|---|---|
| `cargo run --locked -p xtask -- contract verify` | 0 | 9 blobs verified, 0 failures |
| `cargo run --locked -p xtask -- contract check` | 0 | operations.json up to date (59 ops) |
| `cargo run --locked -p xtask -- public-api check` | 0 | 1332 baseline symbols classified |
| `cargo run --locked -p xtask -- forbidden check P00` | 0 | clean |
| `cargo fmt --all -- --check` (1.88.0) | 0 | clean |
| `cargo clippy --locked -p xtask -- -D warnings` (1.88.0) | 0 | clean |
| `git diff --check` | 0 | clean |

### Notes / known limitations at P00

- `nightly-2026-07-10` toolchain alias is not installed as a separate entry;
  the default nightly (`rustc 1.99.0-nightly 2026-07-10`) is the same commit
  and was used for the rustdoc-json public-API snapshot. The exact alias is
  installed by `scripts/bootstrap-tools.sh` consumers / CI.
- Baseline clippy shows 59 `uninlined_format_args` style lints on pre-existing
  0.4 code (clippy-version drift vs plan §2.1; style-only, cleared by P05).
- Several xtask commands (`module-size`, `dep-budget`, `coverage`, `docs`,
  `version`, `examples`, `test-budget`, `tests check-no-ignore`, `fuzz`,
  `sbom`, `future-incompat`, `package`, `release`) are stubs returning exit 2
  until their owning task implements them.
