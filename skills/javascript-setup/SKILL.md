---
name: javascript-setup
version: 1
license: Apache-2.0
prerequisites: node; npm and installed project dependencies when required
helper: build
---
Inspect package.json, the lockfile, entry point and imported modules. Distinguish
CommonJS from ES modules using the declared package type and file extension.
Reproduce the exact script failure in the configured environment. A successful
install does not prove the application works.

Quote the actual missing module, export or source location. Check the dependency
version and public API. Preserve the project's module type, scripts and entry
point unless a migration was requested. Prefer existing named/default exports and
relative paths over compatibility wrappers that hide an import problem.

Use a current symbol/range handle for the smallest coherent patch. Rerun the
original command and relevant behavioral tests. The build helper runs npm test;
check that a test script exists and runs assertions. For entry-point requirements,
also verify stdout, exit status or the actual HTTP/UI behavior through the stated
contract. Compilation or a process merely starting cannot establish acceptance.

For data transformations, inspect whitespace, Unicode, null values and boundary
inputs when relevant. For async behavior, await completion and verify failure
propagation. Keep hypotheses distinct from observed errors. Avoid changes to test
assertions that merely accommodate a broken implementation.

Recovery: restore the implementation checkpoint when an approach fails. If node,
npm, browser binaries or dependencies are missing, show the failed prerequisite
and project-specific setup step. Installation and server processes have external
effects; track them separately from undoable file edits. Report evidence IDs,
current source revision and unresolved behavior.
