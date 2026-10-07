---
name: browser-checks
version: 1
license: Apache-2.0
prerequisites: node; project-local Playwright; installed Chromium; running app
helper: browser
---
Inspect the application route and requested visible behavior. Record the selected
local URL, expected selector/text and optional click. Ensure the application is
running and that the project's browser dependency and binary are available.
Use the browser helper with explicit inputs; it records the actual result and
screenshot. Access mode remains a separate user choice.

Choose selectors based on the current page's accessible role, stable attribute or
known DOM. Do not invent a selector from an unseen screenshot. Check the actual
result after clicking or submitting. A screenshot alone is evidence of appearance,
not an assertion that every interaction works. A successful HTTP response proves
less than a successful user flow.

When a check fails, distinguish an unavailable server, missing browser binary,
selector mismatch, timeout and wrong visible text using the returned evidence.
Inspect relevant source and logs before changing the implementation. Apply a
focused revision-bound edit, restart only owned processes when required, and run
the same check again against the current source. Preserve the original behavioral
expectation unless the user requested a change.

Recovery: retain screenshots and captured output, clean up owned server/browser
processes, and undo unsuccessful source changes with checkpoints. Missing
prerequisites are setup failures, not negative application findings. Bound page
waits and explicitly record cancellations. Report what was clicked, what was
observed, the actual result and remaining untested flows. Do not replace a failed
browser assertion with a model's visual confidence score.
