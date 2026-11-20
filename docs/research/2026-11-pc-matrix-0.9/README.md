# 0.9 PC matrix (order P05)

Reference PC: GTX 1070 8 GB, llama.cpp CUDA 12 build from P04. Suite v5 (8 cases), 16K context,
compact tools, 4,096 output, Guided access, 300 s per case, one repeat. Raw logs stay out of git.

**Every cell ran with Guided checks unavailable.** On this PC Ubuntu restricts unprivileged user
namespaces (`kernel.apparmor_restrict_unprivileged_userns=1`) and there is no AppArmor profile for
bwrap, so every `run_check` returned "Isolation is unavailable … No command ran". Models edited but
never saw a test result. Alt fails closed here by design. Results are not comparable to the 10-08
trial (Trusted access, Spark on Ollama).

## Pass 1 (112 runs)

| Model | Label | Arm | Pass / 8 | Failure classes |
| --- | --- | --- | --- | --- |
| Spark-X2.5 4B Q8 (llama.cpp) | uncensored | trial | 5 | thinking-cutoff 2, timeout 1 |
| Spark-X2.5 4B Q8 (llama.cpp) | uncensored | thinking-off | 6 | wrong-edit 2 |
| Qwen3.5 9B Q4_K_M | standard | trial | 4 | wrong-edit 4 |
| Qwen3.5 9B Q4_K_M | standard | thinking-off | 5 | wrong-edit 3 |
| MiMo 9B Q4_K_M | standard | trial | 5 | wrong-edit 3 |
| MiMo 9B Q4_K_M | standard | thinking-off | 3 | wrong-edit 5 |
| MiMo 9B abliterated Q4_K_M | uncensored | trial | 3 | wrong-edit 5 |
| MiMo 9B abliterated Q4_K_M | uncensored | thinking-off | 3 | wrong-edit 5 |
| `moonshotai/kimi-k3.1-pro` | hosted | trial | 7 | timeout 1 |
| `moonshotai/kimi-k3.1-pro` | hosted | thinking-off | 7 | wrong-edit 1 |
| `minimaxai/minimax-m3.1-flash` | hosted | trial | 5 | provider-fault 3 |
| `minimaxai/minimax-m3.1-flash` | hosted | thinking-off | 2 | provider-fault 4, wrong-edit 2 |
| `z-ai/glm-5.3-flash` | hosted | trial | 0 | timeout 8 |
| `z-ai/glm-5.3-flash` | hosted | thinking-off | 0 | timeout 8 |

Lost tasks by class: wrong-edit 30, timeout 18, provider-fault 7, thinking-cutoff 2.

Pass 2 (two more repeats of the best hosted and best local cell) was stopped part way by the
owner. Repeat 2 only: Kimi 8/8 trial, 7/8 thinking-off (one case killed by the stop); Spark 5/8
trial, 2/8 thinking-off. Repeat 3 not run.

## Findings

- **wrong-edit (30):** with no check able to run, models stop on untested code. In addition, Alt
  rendered the refused check like a failed one: a repair hint plus `Observed failed cases: []`;
  one model read that as a pass. A fix exists as a patch outside the repo, not built yet.
- **rust-feature 0/14:** 8 failures are one edge case. Models average two `i64` values in `f64`
  (avoids overflow, as the prompt asks); the oracle requires the exact `-0.5` for
  `[i64::MIN, i64::MAX]`, which needs `i128`. Suite kept unchanged.
- **timeout (18):** 16 are `z-ai/glm-5.3-flash`, whose replies start after about 285 s (provider
  queueing; 960 tokens then arrive in 14 s). The rest are requests still in flight at 300 s.
- `z-ai/glm-5.3-flash` reported 628 reasoning tokens with `enable_thinking:false` sent.
