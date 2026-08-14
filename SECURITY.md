# Security Policy

## Supported versions

Security fixes are currently provided for the latest published stable release
line. `Cargo.toml`'s `6.1.0` is an unpublished candidate, not the current
registry release.

| Release line | Security support |
| --- | --- |
| `0.6.x` (latest published: `0.6.0`) | Supported |
| Unpublished `6.1.0` candidate and later working-tree changes | Pre-release; no registry support promise |
| Earlier than `0.6.0` | Not supported |

The published-version result above was verified with `cargo search` and
`cargo info`. Both `v6.0.1` release workflow attempts failed. This audit did
recover the corresponding step logs: run `30091854390` rejected its
then-current lightweight tag, while run `30092565276` passed quality,
packaging, SBOM, checksum, and attestation before crates.io rejected OIDC
authentication because no matching Trusted Publisher was configured for
`AnlangA/zai-rs`. The `6.1.0` candidate supersedes that failed tag with a
version greater than `6.0.1`; a future supported release must still configure
that external publisher before its annotated tag can publish. This table
describes the current policy rather than promising indefinite backports.

## Reporting a vulnerability

Please **do not** disclose suspected vulnerabilities in a public issue,
discussion, pull request, or other public channel.

Use GitHub's private vulnerability reporting form:

<https://github.com/AnlangA/zai-rs/security/advisories/new>

You can also reach the form from the repository's **Security** tab by selecting
**Report a vulnerability**. Repository maintainers should use a draft GitHub
Security Advisory for the same private discussion. If the reporting form is not
available, contact [@AnlangA](https://github.com/AnlangA) privately and ask for
a draft Security Advisory; do not include vulnerability details in a public
request.

Include as much of the following as is safe to share:

- the affected version, feature, and component;
- the vulnerability's impact and the conditions needed to trigger it;
- minimal reproduction steps or a proof of concept;
- relevant logs or traces with credentials, tokens, personal data, and other
  secrets removed;
- any known mitigations and your preferred disclosure or credit details.

Do not test against systems or accounts you do not own or have permission to
use. Do not access other users' data, disrupt services, or place live secrets in
the report.

## What to expect

The maintainers will use the private advisory to:

1. acknowledge the report after it has been reviewed and request any missing
   information;
2. reproduce the issue and assess its severity, scope, and affected versions;
3. coordinate a fix and validation privately, including backports for supported
   release lines where required;
4. prepare releases and coordinate the timing and contents of public disclosure,
   including reporter credit when desired.

Response and remediation times depend on the issue's complexity, impact, and
maintainer availability, so this project does not promise a fixed response or
resolution SLA. Please keep the details private until disclosure is coordinated
through the Security Advisory.
