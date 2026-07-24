# Release checklist

The tag workflow packages and verifies `zai-rs`, generates a CycloneDX JSON
SBOM covering dependencies for every Cargo target and SHA-256 checksums,
uploads those three files as workflow evidence,
creates GitHub build-provenance and SBOM attestations, then publishes with a
short-lived crates.io token obtained through GitHub OIDC.

## One-time repository setup

1. Confirm the copyright holder in `LICENSE` with the project owner. The
   `6.0.1` release retains the existing attribution following owner
   confirmation on 2026-07-24.
2. On crates.io, configure a trusted publisher for:
   - repository: `AnlangA/zai-rs`
   - workflow: `release.yml`
   - environment: `crates-io`
3. In GitHub, protect the `crates-io` environment with exact release-tag
   rules. Add a required reviewer when governance calls for manual approval.
   Keep the workflow's `id-token: write` permission scoped to the publish job.
4. After one successful OIDC publication, remove any legacy
   `CRATES_IO_TOKEN` repository/environment secret.
5. Enable GitHub private vulnerability reporting for the repository; the
   checked-in `SECURITY.md` describes the policy but cannot enable that setting.

Trusted publishing setup is documented by
[crates.io](https://crates.io/docs/trusted-publishing) and the temporary-token
action is maintained by
[`rust-lang/crates-io-auth-action`](https://github.com/rust-lang/crates-io-auth-action).

## Per-release procedure

Published crates.io versions are immutable and cannot be reused. Confirm that
the version in `Cargo.toml` is still available before creating a tag; the
observable behavior changes listed in `HARDENING_MIGRATION.md` must inform that
version decision.

1. Make the intended version explicit in `Cargo.toml` and update release notes.
2. Run the quality gates in `docs/OPTIMIZATION_PLAN.md` from a clean checkout.
3. Review `cargo package --locked -p zai-rs --all-features --list`; confirm no
   credentials, fixtures, build output, or repository-only tooling is present.
4. Create an **annotated** tag whose name exactly matches the crate version,
   for example `git tag -a v6.0.1 -m "zai-rs 6.0.1"`, then push that tag.
5. Wait for the reusable CI job to succeed. If the `crates-io` environment
   requires approval, approve it only after that point.
6. Download the `zai-rs-<version>-release-evidence` workflow artifact and run
   `sha256sum -c SHA256SUMS` from its directory.
7. Verify the provenance against this repository, for example:

   ```bash
   gh attestation verify zai-rs-6.0.1.crate --repo AnlangA/zai-rs
   ```

Setting the GitHub Actions configuration variable `SKIP_PUBLISH` to `true`
packages and uploads the evidence artifact but skips crates.io authentication,
publication, and GitHub attestations. It is intended for validating release
automation before a real tag publication. The job still targets the
`crates-io` environment, so that environment's protection rules may still
require approval.

Crates.io releases are immutable. If a published version is defective, yank it,
fix forward with a new version, and preserve the evidence from both workflow
runs.
