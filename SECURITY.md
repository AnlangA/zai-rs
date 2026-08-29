# Security Policy

## Supported versions

Security fixes are currently provided for the latest published stable release
line.

| Release line | Security support |
| --- | --- |
| `6.2.x` (latest: `6.2.0`) | Supported |
| Earlier than `6.2.0` | Not supported |

Both `v6.0.1` release workflow attempts failed. This audit did
recover the corresponding step logs: run `30091854390` rejected its
then-current lightweight tag, while run `30092565276` passed quality,
packaging, SBOM, checksum, and attestation before crates.io rejected OIDC
authentication because no matching Trusted Publisher was configured for
`AnlangA/zai-rs`. The annotated `v6.1.0` tag then triggered run `31818272628`:
attempt 1 failed in the macOS quality job because of an accept-vs-read test
race, and attempt 2 passed the internal release gates before the same missing
Trusted Publisher configuration rejected OIDC authentication. Version `6.1.0`
was published afterward, but those failed workflow attempts remain part of the
release record. Version `6.2.0` uses a new annotated tag and a matching
crates.io Trusted Publisher. This table describes the current policy rather
than promising indefinite backports.

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
