---
name: configuration-data
version: 1
license: Apache-2.0
prerequisites: project's existing runtime and behavioral checks
helper: build
---
Inspect the configuration schema, defaults and its actual consumers. Reproduce
the failure through a real caller. Separate missing fields, wrong types, malformed
data and unavailable resources. Preserve false, zero, empty and null values when
the contract permits them. Test bounds, Unicode, escaping and rejected inputs.
Use revision-bound edits and rerun unchanged assertions. For persistent data,
verify failure recovery and atomic replacement rather than only the successful
path. Missing dependencies are setup facts. Report current evidence, source
revision and unresolved behavior; do not certify a fix from valid JSON alone.
