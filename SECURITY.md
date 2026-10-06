# Security reports and execution boundaries

The current maintained development line is 0.5 beta. Fixes target `main`; older
experimental versions do not have a separate support commitment.

For a vulnerability that exposes private data or enables unintended execution,
use GitHub's private vulnerability reporting for this repository when available.
If it is not enabled, open an issue asking the maintainer for a private reporting
channel without including the sensitive details. Keep credentials and sensitive
reproduction details out of public issues.

Include the affected commit/version, the selected access mode, a minimal
reproduction using disposable data, expected versus observed behavior and whether
the issue affects saved state. Report malformed protocol, path-boundary, artifact
integrity, authorization and process-ownership failures with the same specificity.

Full access intentionally allows arbitrary commands, network access and external
tools using normal account permissions. It is not a hostile-code sandbox.
Guided checks require Bubblewrap and fail closed when unavailable. Model output,
project content and external tool responses are untrusted inputs; they do not
establish successful verification. File undo and state backup do not reverse
arbitrary host or remote effects.

Conversation exports and tool logs may contain project data. API keys are referenced
by environment-variable name; avoid including secret values in issue attachments.
See [verification contracts](docs/VERIFICATION_CONTRACTS.md) and
[third-party provenance](THIRD_PARTY.md) for the implemented boundaries.
