# Security reports and execution boundaries

The current maintained line is 0.6 beta (latest published `v0.6.0-beta.3`), with
0.7 in development on `main`. Fixes target `main`; older experimental versions do
not have a separate support commitment.

For a vulnerability that exposes private data or enables unintended execution,
use GitHub's private vulnerability reporting for this repository. The repository
owner must switch it on (Settings → Code security → Private vulnerability
reporting); until then, open an issue asking the maintainer for a private
reporting channel without including the sensitive details. Keep credentials and sensitive
reproduction details out of public issues.

Include the affected commit/version, the selected access mode, a minimal
reproduction using disposable data, expected versus observed behavior and whether
the issue affects saved state. Report malformed protocol, path-boundary, artifact
integrity, authorization and process-ownership failures with the same specificity.

Full access intentionally allows arbitrary commands, network access and external
tools using normal account permissions. It is not a hostile-code sandbox.
"Allow this session" in the TUI and `--allow-tools` approve project actions only
(checkpointed edits, configured checks, skill helpers). Terminal commands and
external MCP tools need their own opt-in: "Allow terminal this session" or
`--allow-terminal`. Project tools never read or edit Alt's own data folder, and
credential files (`.env*`, `.envrc`, `.netrc`, `.npmrc`, `.pypirc`, SSH keys,
`credentials*.json`, `.docker/`, `*.pem`, `*.key`) are excluded whatever their case.
The owned llama-server listens on loopback with a random per-launch key.
Guided checks require Bubblewrap and fail closed when unavailable. Model output,
project content and external tool responses are untrusted inputs; they do not
establish successful verification. File undo and state backup do not reverse
arbitrary host or remote effects.

Conversation exports and tool logs may contain project data. API keys are referenced
by environment-variable name; avoid including secret values in issue attachments.
See [verification contracts](docs/VERIFICATION_CONTRACTS.md) and
[third-party provenance](THIRD_PARTY.md) for the implemented boundaries.
