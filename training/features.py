"""Assistant-target SFT encoding with exact native prompt-prefix checks.

One sample per assistant decision. All earlier user, tool and assistant context
is masked. No generation tags, fabricated reasoning or silent truncation.
"""
from copy import deepcopy

from prepare import DataError, require


def render(tokenizer, messages, tools, template_kwargs, generation):
    return tokenizer.apply_chat_template(
        messages, tools=tools or None, tokenize=True,
        add_generation_prompt=generation, **template_kwargs,
    )


def encode_targets(tokenizer, record, max_length, template_kwargs=None):
    template_kwargs = template_kwargs or {}
    require(not {"tools", "tokenize", "add_generation_prompt", "return_dict"}.intersection(template_kwargs), "template kwargs cannot override encoding controls")
    targets = []
    for index, message in enumerate(record["messages"]):
        if message["role"] != "assistant":
            continue
        prefix = record["messages"][:index]
        complete = record["messages"][:index + 1]
        prompt_ids = render(tokenizer, prefix, record["tools"], template_kwargs, True)
        full_ids = render(tokenizer, complete, record["tools"], template_kwargs, False)
        label = f"{record['id']}:assistant-{index}"
        require(isinstance(prompt_ids, list) and isinstance(full_ids, list), f"{label}: tokenizer must return unbatched token IDs")
        require(all(type(x) is int for x in prompt_ids + full_ids), f"{label}: invalid token IDs")
        require(bool(prompt_ids), f"{label}: empty native prompt")
        require(full_ids[:len(prompt_ids)] == prompt_ids, f"{label}: native generation prompt is not an exact completion prefix; qualify template/settings before training")
        require(len(full_ids) > len(prompt_ids), f"{label}: native template produced no assistant target")
        require(len(full_ids) <= max_length, f"{label}: {len(full_ids)} tokens exceed {max_length}; re-collect shorter evidence or explicitly change qualified context; no truncation applied")
        # A template that silently discards native calls/reasoning cannot teach them.
        for call_index, _ in enumerate(message.get("tool_calls", [])):
            for field in ("name", "arguments"):
                probe = deepcopy(complete)
                function = probe[-1]["tool_calls"][call_index]["function"]
                if field == "name":
                    function[field] = function[field] + "__alt_probe__"
                else:
                    function[field] = {**function[field], "__alt_probe__": "serialization-check"}
                require(render(tokenizer, probe, record["tools"], template_kwargs, False) != full_ids, f"{label}: native template ignores tool {field}")
        if message.get("reasoning_content"):
            probe = deepcopy(complete)
            probe[-1]["reasoning_content"] += " __alt_reasoning_probe__"
            require(render(tokenizer, probe, record["tools"], template_kwargs, False) != full_ids, f"{label}: native template ignores recorded reasoning_content")
        targets.append({
            "input_ids": full_ids,
            "attention_mask": [1] * len(full_ids),
            "labels": [-100] * len(prompt_ids) + full_ids[len(prompt_ids):],
        })
    require(bool(targets), f"{record['id']}: no assistant targets")
    return targets


def padded_batch(features, pad_token_id):
    """Right padding; padding and every prompt token remain outside the loss."""
    require(bool(features), "empty training batch")
    width = max(len(row["input_ids"]) for row in features)
    batch = {key: [] for key in ("input_ids", "attention_mask", "labels")}
    for row in features:
        missing = width - len(row["input_ids"])
        batch["input_ids"].append(row["input_ids"] + [pad_token_id] * missing)
        batch["attention_mask"].append(row["attention_mask"] + [0] * missing)
        batch["labels"].append(row["labels"] + [-100] * missing)
    return batch
