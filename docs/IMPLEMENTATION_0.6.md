# Implementation and verification — Alt 0.6 beta

This pass turns the readiness review into the [0.6 work orders](WORK_ORDERS_0.6.md).
It remains a beta. Application checks, model task success, physical hardware
qualification and actual beginner usability are separate results.

## Delivered behavior

- Available-action instructions now match the selected tool focus, access and
  exact configured check names. These instructions sit outside trimmed memory.
  A structured-evidence error marks a tool call failed even if its process exits 0.
- Home offers **Prepare project checks**. Suggestions respect declared JavaScript
  package managers/test scripts and pytest configuration. Asking to run an empty
  check list opens setup. Nothing silently expands the selected tools or access.
- **Try a practice project** creates a fresh example with four independently
  pinned Python behavioral checks. It demonstrates a real failing seed, repair,
  saved evidence, restart and undo. `alt practice` also creates an example.
- Small terminals prioritize verification status over a long task description.
- Memory presents complete source excerpts, records omitted excerpts, retrieves
  explicitly named files and filters conversational filler. Token-budget trimming
  reassembles memory rather than cutting through an excerpt. Current evidence
  retains priority over historical notes; native context is not enlarged.
- Interrupted downloads finish pending writes before reporting a resumable
  offset. Repeated real HTTP truncations exercise this boundary.
- The Goose 1.53 adapter recognizes its synthetic network-failure message and
  reports an interrupted turn instead of successful completion. Partial streamed
  content is drained into history before returning an error. Recognition is
  specific to the pinned engine's message shape; other engines need their own
  protocol qualification.
- Package updates back up the selected state before replacing an existing
  executable, preserve the previous executable and abort if backup cannot be
  created. The package includes the local PC test recorder.
- Release tooling supports GitHub identity-signed provenance, exact package
  verification and a prior-CI-artifact upgrade gate without a long-lived signing
  secret. The existing private-key candidate workflow remains available.

## Verification record

Results for this candidate are being collected. No pending check below is a pass.

- Targeted practice, tool-guidance, retrieval and download-recovery Rust checks
  have passed; final whole-tree checks are recorded in the evidence index.
- The real practice PTY journey passes at 120×40, 80×24 and 60×18, including
  persisted verification after restart and return to the original failure after undo.
- The streaming suite passed 60 scenarios across 12 repeated normal/disconnect/
  cancel/engine-crash/reconnect cycles with actual Alt and Goose processes.
  The HTTP provider is deterministic and contains no model weights.
- The complete uncensored matrix records 100 development and 20 held-out attempts
  on one frozen executable. A failed behavioral task remains a failure even if
  the model claims completion or the engine returns `end_turn`. Candidate pilots
  and interrupted campaigns remain separate from that matrix.

The final evidence index and measured matrix totals will be added after the
campaign, package gate and remote checks finish. No recommended preset or broad
coding-reliability claim follows from an application test pass.

## Outside cloud acceptance

The physical GTX 1070/8 GB VRAM/16 GB RAM Linux computer, other actual older PCs
and newer GPUs still need measurement. Follow [PC testing](PC_TESTING.md).
Actual uncoached beginner sessions require people; terminal automation does not
replace the [novice protocol](NOVICE_ACCEPTANCE.md). Private/gated model acquisition
requires applicable credentials, and real LM Studio remains a separate connection
qualification. The cloud does not supply those observations.

See [0.5 evidence](IMPLEMENTATION_0.5.md) for the preceding snapshot. Its statements
about unpublished source and unrun remote CI describe that earlier point in time;
the 0.5 source and successful GitHub CI were subsequently published.
