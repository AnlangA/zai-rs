# Release checklist

The tag workflow packages and verifies `zai-rs`, generates a CycloneDX JSON
SBOM covering all crate features across all target platforms and SHA-256 checksums,
uploads those three files as workflow evidence,
creates GitHub build-provenance and SBOM attestations, then publishes with a
short-lived crates.io token obtained through GitHub OIDC.

## Current release state

`Cargo.toml` declares `6.1.0`, a release candidate prepared on 2026-08-14 that
supersedes the never-published `6.0.1`. It carries the API-transport hardening
from PR #57 and GLM-5.3 model support. The two `v6.0.1` workflow attempts,
runs `30091854390` and `30092565276`, both concluded with `failure`. Their
step logs are available: the first run's then-current `v6.0.1` ref failed the
annotated-tag check; the second passed quality, tag verification, packaging,
SBOM, checksum, and both attestation steps, then crates.io rejected OIDC
authentication with `No Trusted Publishing config found for repository
AnlangA/zai-rs`.

`cargo search` and `cargo info` confirm `0.6.0` as the latest crates.io
release. Publishing `6.1.0` therefore still requires the crates.io Trusted
Publisher named in the one-time setup below to be configured before the
`v6.1.0` annotated tag is pushed; the existing `v6.0.1` tag must not be moved
or reused.

The `v6.1.0` tag was pushed on 2026-08-14 (run `31818272628`). Attempt 1
failed when the macOS quality job hit an accept-vs-read race in the
`proxy_isolation` test helper; attempt 2 passed every internal gate — quality,
annotated-tag verification, packaging, SBOM, checksum, and both attestations —
and then failed only at the crates.io authentication step with the same
`No Trusted Publishing config found for repository AnlangA/zai-rs` rejection.
The tag itself is valid and must not be moved; once the Trusted Publisher is
configured, rerun the failed `publish` job of run `31818272628` to complete
the release.

## One-time repository setup

1. Confirm the copyright holder in `LICENSE` with the project owner. The
   unpublished `6.0.1` candidate retained the existing attribution following
   owner confirmation on 2026-07-24; repeat the check if release ownership or
   included material changes.
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
version decision. The current `6.1.0` candidate already satisfies the recovery
requirement of a version greater than the unpublished `6.0.1`.

1. Make the intended version explicit in `Cargo.toml`, update `Cargo.lock`, and
   update README, installation, migration, security, and release notes together.
2. Run the repository quality gates mirrored by `.github/workflows/ci.yml`
   from a clean checkout.
3. Run `./scripts/verify-package-contents.sh`; it inspects the actual `.crate`
   archive and rejects repository-only paths, non-regular entries, missing
   contract files, and unexpectedly large package growth without freezing an
   exact self-referential file count. Review
   `cargo package --locked -p zai-rs --all-features --list` as a human
   cross-check.
4. Create an **annotated** tag whose name exactly matches the crate version,
   for example `git tag -a v6.1.0 -m "zai-rs 6.1.0"`, then push that new tag.
5. Wait for the reusable CI job to succeed. If the `crates-io` environment
   requires approval, approve it only after that point.
6. Do not call the release complete merely because the tag exists or the
   quality job is green. Complete all post-release verification below.

## Failed-tag recovery

1. Preserve the failed tag, run URL, run id, available artifacts, and logs. Do
   not delete, force-move, or reuse a tag to hide a failed attempt.
2. Retrieve the failing job and step logs. If they are unavailable, record the
   root cause as unknown; do not guess from the run-level conclusion.
3. Correct the identified cause in its owning system and rerun all release
   quality gates from a clean checkout. For the known `v6.0.1` failure, create
   the crates.io Trusted Publisher named in the one-time setup; do not add or
   expose a long-lived registry token as a workaround.
4. Because the current tree differs from `v6.0.1`, bump the package to a version
   greater than `6.0.1`. Update trusted-publisher and protected-environment tag
   rules so the new tag is permitted.
5. Create and push a new annotated tag pointing at the fully audited commit.
   Preserve the two failed `v6.0.1` runs as release-history evidence.

## Post-release verification

After the publish step reports success:

1. From outside the repository checkout (to prevent the local package from
   influencing resolution), verify that crates.io resolves the exact version,
   not merely the crate name or a cached local checkout. Replace `<version>`
   with the published version:

   ```bash
   cargo search zai-rs --limit 1
   cargo info --registry crates-io zai-rs@<version>
   ```

2. Open `https://docs.rs/zai-rs/<version>/zai_rs/` and confirm that the exact
   version built successfully with the intended feature surface.
3. Download the
   `zai-rs-<version>-release-evidence-<run_id>-<run_attempt>` workflow artifact
   and run `sha256sum -c SHA256SUMS` from its directory. The run id/attempt
   suffix makes every rerun explicitly identifiable and avoids depending on
   GitHub's cleanup or same-name behavior for immutable artifacts.
4. Verify provenance against this repository, for example:

   ```bash
   gh attestation verify zai-rs-<version>.crate --repo AnlangA/zai-rs
   ```

5. Confirm that the evidence artifact contains the exact `.crate`, CycloneDX
   SBOM, and checksums named for the published version. Record the registry
   version, docs.rs URL, workflow run, evidence artifact, and attestation result
   in the release notes.

Use the reusable CI `publish-dry-run` job to rehearse packaging. A tag-triggered
Release intentionally has no skip-publication mode: it can only turn green
after provenance/SBOM attestations, crates.io OIDC authentication, and
publication all succeed. This prevents a persistent environment variable from
making an unpublished tag look like a completed release.

Crates.io releases are immutable. If a published version is defective, yank it,
fix forward with a new version, and preserve the evidence from both workflow
runs.
