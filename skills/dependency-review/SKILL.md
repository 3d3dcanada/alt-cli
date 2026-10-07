---
name: dependency-review
version: 1
license: Apache-2.0
prerequisites: locked dependency inventory; pip-audit or npm
helper: pip-audit or npm-audit
---
Inspect the lockfile and installed dependency versions. Select the Python or
JavaScript audit helper explicitly. Preserve raw advisory output and distinguish
an unavailable scanner, skipped dependencies or zero scanned packages from a
clean result. Review advisory applicability and compatible fixed versions before
editing. Rerun the audit and actual application tests after a dependency change.
For source review, the semgrep pack is available separately; a finding requires
review and a reproduction. Neither a scanner's formatting nor a model's agreement
verifies a vulnerability or proves its absence. Record evidence and unresolved
coverage. Helpers retain the user's existing execution access choice.
