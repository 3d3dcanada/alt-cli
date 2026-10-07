---
name: python-repair
version: 1
license: Apache-2.0
prerequisites: python3; project test dependencies already installed
helper: build
---
Inspect the relevant module and its caller. Read the project's configured checks;
prefer its existing runner and environment. Reproduce the reported failure before
editing. Quote the actual traceback location and failing assertion. A guessed cause
is a hypothesis, never a fact.

For import failures, inspect package layout and the installed dependency version.
Use an existing bundled helper when requested. Do not copy its implementation or
install an unrelated package to make an import succeed. For paths, distinguish the
current directory from the module's directory. For data, check empty, boundary,
Unicode, duplicate and invalid-input behavior against the requested contract.

Use a revision-bound symbol or range handle for a focused implementation change.
Preserve existing tests. Reread after any stale-handle failure. Check syntax using
the editor feedback, then run the actual configured behavioral check. The build
helper can execute a standard Python unittest discovery command in Full access;
it does not know a project's custom test contract. A zero-test run proves nothing.

On failure, inspect the new traceback and update the hypothesis instead of
repeating identical calls. Missing prerequisites should produce a useful setup
step. Never claim a fix from a code block or successful import alone. Report the
changed paths, real evidence IDs, actual test scope and remaining uncertainty.

Recovery: checkpoint undo restores the implementation if the approach fails;
terminal side effects require separate recovery. A passing earlier check is stale
after source, environment or assertion changes. Keep the full captured evidence
available and send only the relevant diagnostic lines back into the model context.
