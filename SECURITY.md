# Security Policy

## Supported versions

Kamino is pre-1.0 and ships from `main` only. Security fixes land on `main`; there are no
maintained release branches yet. Always test against the latest commit.

## Reporting a vulnerability

Do **not** open a public issue, discussion or pull request for a security problem.

Report it privately through GitHub Security Advisories:

**https://github.com/preskater/kamino/security/advisories/new**

If you cannot use advisories, contact the maintainers listed in [CODEOWNERS](CODEOWNERS)
through GitHub.

Please include:

- affected service (`agents`, `projects`) and endpoint, or the shared crate
- commit SHA or version
- reproduction steps, ideally a `curl` request against a local instance
- impact assessment (data exposure, denial of service, SQL injection, auth bypass, …)
- logs with `RUST_LOG=debug` if relevant, with secrets removed

## What to expect

- Acknowledgement within 3 business days.
- An initial assessment and severity classification within 7 days.
- A fix and coordinated disclosure date for confirmed issues. Credit in the advisory unless
  you ask to stay anonymous.

## Scope

In scope:

- SQL injection or missing authorization in the HTTP APIs
- denial of service through unbounded input, pagination or query parameters
- unsafe handling of uploaded content, paths or URLs
- dependency vulnerabilities reachable from the services
- secret leakage in logs or error responses

Out of scope:

- missing authentication/authorization on a local development instance where none is
  configured (the APIs currently ship without auth by design)
- reports from automated scanners without a reproducible impact
- issues in dependencies that are not reachable from this codebase

## Safe harbour

We will not pursue legal action for good-faith security research that follows this policy,
avoids data destruction and privacy violations, and gives us reasonable time to remediate
before public disclosure.

## Dependencies

Dependabot alerts and the `audit` CI job (`cargo-audit`) watch the dependency tree. Please
still report reachable issues directly if you find one first.
