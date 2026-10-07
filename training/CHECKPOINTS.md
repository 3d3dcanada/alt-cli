# 7B/9B checkpoint and architecture notes

October 7, 2026. These are inspected candidates to illustrate the handoff, not
automatic model selections. The owner's exact 7B and 9B artifacts have not been
specified. Keep their GGUF hashes and locate matching HF training weights before
filling the configs. Publisher labels/licenses require their own review.

| Candidate | Inspected immutable revision | What is established here |
|---|---|---|
| [Josiefied Qwen2.5-7B abliterated](https://huggingface.co/Goekdeniz-Guelmez/Josiefied-Qwen2.5-7B-Instruct-abliterated/tree/a6b1fb11f7096463d7e0792c36543869a83f2ed7) | `a6b1fb11f7096463d7e0792c36543869a83f2ed7` | HF checkpoint listing, publisher Apache-2.0 field, `qwen2` / `Qwen2ForCausalLM`, native tool template and tokenizer available; tokenizer-only fixture receipt is separate |
| [MiMo-V2.6-Distill-Qwen-9B Heretic](https://huggingface.co/saidutta69/MiMo-V2.6-Distill-Qwen-9B-heretic/tree/38e79d42c4995ca25b4b397872f0bbb4cd652175) | `38e79d42c4995ca25b4b397872f0bbb4cd652175` | HF parent of the already researched GGUF; publisher MIT field; config is `qwen3_5` / `Qwen3_5ForConditionalGeneration`, not the starter's older text-only causal-LM architecture |

Cards/configs/template metadata and their hashes are in the handoff source ledger.
No weights from these HF checkpoints were downloaded or trained in this work.
The 7B tokenizer fixture is not a native runtime tool round trip or a model-quality
test. The previously inspected MiMo Q4 artifact is **5,629,106,560 bytes** before
cache/buffers; parameter count alone is not an accurate fit estimate.

The quantizer's other Qwen2.5-7B card links
`huihui-ai/Qwen2.5-7B-Instruct-abliterated`; its metadata request returned HTTP 401
here. That does not establish its current public accessibility or training-file
availability. Do not replace that exact derivative with Josiefied merely because
both are abliterated 7B models. Adapter ancestry must match exactly.

## MiMo 9B training handoff

The supplied Transformers 4.57.1 starter predates the inspected Qwen3.5 type and
must not be presented as a working MiMo 9B trainer. Current package metadata lists
newer Transformers releases, but a latest version number is not qualification.
H11 must pin a model-compatible modern stack and preserve the conditional model's
architecture. The actual derivative stores its template in a separate Jinja file
and supports explicit thinking/tool serialization; capture that file's hash too.

The architecture-specific qualification work is:

1. Pin a Transformers/PEFT/PyTorch/CUDA combination supporting the exact
   `Qwen3_5ForConditionalGeneration` config. Load config, tokenizer/template and
   required processor artifacts without remote code; render recorded text/native
   tool fixtures. Use the same template as deployment, including thinking controls.
2. Qualify the derivative card's `AutoModelForImageTextToText`/`AutoProcessor`
   example for training instead of `AutoModelForCausalLM`. Verify text-only forward/loss with unused modalities
   absent. Preserve required vision/processor artifacts; do not drop modules or
   remap weights based on guessed names.
3. Inspect actual module names. Choose/freeze modalities and LoRA targets explicitly;
   audit trainable parameters. Prove user/tool masks, a finite loss, nonzero adapter
   gradients and one optimizer update on the selected GPU. Measure NF4 support,
   precision, activation peaks and save/load behavior. None has run here.
4. Run the same reviewed-corpus pilot and before/after behavioral evaluation.
   Merge into the exact high-precision derivative; test architecture-compatible
   GGUF conversion, companion artifacts, tool parser and Pascal inference.

Other supported text-only 9B families can use the family-neutral config after
their own model terms, native tools, prefix masks and hardware qualification.
Model size does not determine which loader or training recipe is correct.
