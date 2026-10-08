# Choosing and measuring a local runtime

Alt keeps the selected model and runtime explicit. If a runtime or agent-engine
path chosen in Settings is missing or is not executable, repair that path or
choose another executable. Alt does not silently switch to a discovered copy.

For the GTX 1070, use the existing Pascal `sm_61` source-build instructions with
CUDA 12, or retain CPU operation. A detected NVIDIA card or a requested GPU layer
count does not establish that a selected runtime can execute its kernels.

## Inspect and propose settings

`alt models inspect /path/to/model.gguf` reads bounded GGUF structure and metadata
without generating text or loading model weights. Imports validate metadata,
tensor descriptors and known tensor payload sizes. Numbered split imports discover
the complete set, verify every member, record aggregate size, and retain all
original files. Missing, inconsistent, truncated or changed members must be
repaired before loading. A tensor encoding whose exact byte layout is unknown to
the inspector is explicitly reported by `tensor_payload_sizes_verified: false`;
the selected runtime must still support that encoding and architecture.

`alt hardware --fit` proposes settings for the selected local artifact, using its
declared architecture/context/KV shape and separately observed RAM and first-device
VRAM availability. Proposals do not change configuration. KV estimates assume
standard full attention across the declared layers; hybrid models can differ.
Unknown metadata remains unknown. Buffer reserves and approximate layer fractions
are estimates, so a proposal that fits the estimate still needs calibration.

Use 4K or 8K context initially for a selected 7B/9B Q4 model. Apply the desired
context, GPU layers, batch and cache settings explicitly, then disconnect/reconnect
and run a benchmark. Changing context in an external profile does not by itself
change that server's native window. `alt hardware --provider` records supported
provider metadata where available and labels missing observations.

## Measure the selected configuration

`alt benchmark` records the exact selected artifact/runtime, loading and integrity
time, three generation samples, observed first generated output, server-reported
prompt/decode rates when supplied, and sampled peak owned-process RAM/VRAM.
The first sample follows a new owned load; later samples reuse the same process
after file-identity checks. External servers' cold/warm state and memory remain
unmeasured. A non-streaming response cannot provide first-token timing.

Actual GPU layer counts come from recognized runtime log output. Missing counts
remain unmeasured; Alt never presents the requested layer setting as observed
offload. Sampling can miss short memory peaks. Throughput results do not establish
coding correctness: complete the practice/independent-check journey separately.

Model loading defaults to 180 seconds. Change **Settings → Model and runtime
settings → Loading deadline**, or use `alt runtime --startup-timeout-secs 300`.
The range is 5–3600 seconds. Cancellation and deadline expiry stop the owned
process without selecting a different model or silently extending the deadline.

## Ownership and private local access

Chat, benchmark and qualification transitions release the earlier owned runtime
before loading a replacement. A state-directory lease prevents competing Alt
windows from double-loading owned models in that state. An external Ollama or
other server retains its operator's separate ownership.

The managed runtime listens on a Unix socket inside a private per-launch
directory. Alt retains a loopback listener with an unpredictable per-launch URL
prefix for its clients. Readiness uses the private socket, so another TCP listener
cannot impersonate a successfully loaded owned runtime. Owned requests bypass
inherited HTTP proxies. Custom owned runtimes must support the pinned llama.cpp
Unix-socket interface; otherwise connect to them as explicitly managed external
endpoints.

An owned conversation reuses its already verified runtime between user turns.
Changes to the observed model or executable file identity interrupt reuse and
require verification/reconnection. New launches always verify all recorded model
hashes. These checks describe observed filesystem identity at the boundary; they
do not promise immutability against unrelated writers during an active inference.

Engine and MCP cleanup retain process-group ownership after the group leader
exits, then use bounded termination, kill and leader reaping. Full access continues
to permit arbitrary host commands and external side effects. Deliberately detached
processes and externally operated servers are outside an owned process group's
cleanup guarantee. Extension discovery itself requires effective Full access,
including CLI discovery; Review/Guided discovery starts no server or HTTP request.

The cloud fixture checks cover these transitions and malformed inputs. Actual
GTX 1070 VRAM fit/speed, another older CPU machine, and a newer GPU remain physical
hardware measurements; no GPU preset is promoted from CPU or protocol fixtures.
