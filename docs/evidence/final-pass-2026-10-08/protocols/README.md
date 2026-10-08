# Final headless protocol validation

All **12 selected jobs passed** on frozen Alt SHA-256 `293ab2625a5896c8bcd6c94c9ee3848e460dd3fdbf6fb94f23803815ad410172`. These deterministic providers and real tool/check executions establish application behavior, not model coding quality or GTX 1070 performance.

`SOURCE_RECEIPT.json` contains commands, exact initial source hashes, binary/Goose identities, all 13 attempts and the 12 final selections. The application source hashes still match the later checkout. Its initial checkout HEAD and later finalization HEAD have distinct meanings; neither replaces the recorded binary hash.

The first stream-recovery attempt failed because its disconnect completed before process ownership was sampled. `initial-stream-attempt/` and `SOURCE_RECEIPT.initial.json` preserve that failure. A fixture-only handshake synchronized this observation; the unchanged application then passed all ten streaming scenarios across two rounds. The rerun preserves failed-stream, history, cancellation, crash, reconnect and process-cleanup assertions.

Coverage: five executed skills including headless Chromium; bounded analysis and timeout accounting; native tool-result challenge with negative/positive cases; independent Rust acceptance and evidence collection; Goose OpenAI and authenticated Ollama; compact and compact-lines on both providers with 3/12-turn exact request retention and dirty Git preservation; candidate selection/promotion/undo; and streaming recovery.

The bundle retains bounded text logs and raw JSON/JSONL. `INVENTORY.json` lists every original file, its hash and any exclusion. The small browser screenshot and generated orchestrator helpers are excluded from this textual bundle; their hashes remain in the inventory. `SHA256.json` covers every retained artifact except itself.
