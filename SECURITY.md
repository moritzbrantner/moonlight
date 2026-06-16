# Security Policy

## Supported Versions

Security fixes target the current `main` branch and the latest published `0.x`
release.

## Reporting A Vulnerability

Report vulnerabilities privately through GitHub security advisories when
available, or contact the maintainer directly before opening a public issue.
Include the affected version, reproduction steps, expected impact, and any
relevant configuration.

## Local-First Safety Model

`moonlight-http` duplicates traffic to primary, candidate, and optional
secondary targets. It is intended for trusted local development, isolated demo
services, and disposable test environments. Do not shadow non-idempotent
production traffic unless the targets are explicitly safe to receive duplicate
requests.

Sensitive headers are redacted by default. Request and response bodies are still
captured as hashes plus previews unless JSON body redaction is configured with
`[comparison].redact_json_paths` in `moonlight.conf` or `--redact-json-path`.
Moonlight is not a data-loss-prevention system;
avoid sending secrets or regulated data through comparison runs.
