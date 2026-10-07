#!/usr/bin/env python3
"""Weight-free native-template fixture, never a model/tool-capability benchmark."""
import argparse
import hashlib
import importlib.metadata
import json
import sys
from pathlib import Path

from features import encode_targets
from prepare import DataError, REVISION, require, sha256_file


def fixture():
    return {
        "id": "synthetic-serialization-fixture", "synthetic": True,
        "tools": [{"type": "function", "function": {
            "name": "read", "description": "Read a file", "parameters": {
                "type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"],
            },
        }}],
        "messages": [
            {"role": "system", "content": "Use actual evidence."},
            {"role": "user", "content": "Read café.py and inspect the Unicode identifier Δ."},
            {"role": "assistant", "content": None, "tool_calls": [{
                "id": "call-1", "type": "function", "function": {"name": "read", "arguments": {"path": "café.py"}},
            }]},
            {"role": "tool", "tool_call_id": "call-1", "content": "fixture a: return 1"},
            {"role": "assistant", "content": "The recorded fixture check passed."},
        ],
    }


def qualify(args):
    require(REVISION.fullmatch(args.revision), "pin the tokenizer checkpoint revision")
    require(importlib.metadata.version("transformers") == "4.57.1", "qualify another Transformers version separately")
    from transformers import AutoTokenizer
    location = args.local_tokenizer or args.repository
    kwargs = {"local_files_only": True} if args.local_tokenizer else {"revision": args.revision, "cache_dir": str(args.cache_dir)}
    tokenizer = AutoTokenizer.from_pretrained(location, use_fast=True, trust_remote_code=False, **kwargs)
    row = fixture()
    template_kwargs = json.loads(args.template_kwargs)
    require(isinstance(template_kwargs, dict), "template controls must be a JSON object")
    receipt = {
        "schema_version": 1, "repository": args.repository, "revision": args.revision,
        "mode": "tokenizer_only_with_synthetic_fixture",
        "model_weights_loaded": False, "training_run_performed": False,
        "live_tool_execution_performed": False, "template_kwargs": template_kwargs,
        "packages": {name: importlib.metadata.version(name) for name in ("transformers", "tokenizers", "jinja2", "huggingface-hub")},
        "native_template_sha256": hashlib.sha256(tokenizer.get_chat_template(tools=row["tools"]).encode()).hexdigest(),
        "limits": "One tokenizer/template fixture. Does not establish model quality, native runtime parser compatibility, training compatibility, or hardware fit.",
    }
    if args.local_tokenizer:
        filenames = ("config.json", "tokenizer.json", "tokenizer_config.json", "chat_template.jinja", "special_tokens_map.json", "vocab.json", "merges.txt", "tokenizer.model")
        receipt["tokenizer_files"] = {name: sha256_file(args.local_tokenizer / name) for name in filenames if (args.local_tokenizer / name).is_file()}
        receipt["provenance"] = "Local files hashed; repository/revision are caller-declared and require comparison with the source ledger."
    try:
        targets = encode_targets(tokenizer, row, 2048, template_kwargs)
        receipt.update({
            "status": "passed_prefix_mask_and_tool_serialization_fixture", "assistant_targets": len(targets),
            "input_tokens": [len(target["input_ids"]) for target in targets],
            "supervised_tokens": [sum(value != -100 for value in target["labels"]) for target in targets],
        })
    except DataError as error:
        receipt.update({"status": "failed", "error": str(error)})
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--uncensored", required=True, action="store_true", help="Explicit candidate selection; no inference is run")
    parser.add_argument("--local-tokenizer", type=Path)
    parser.add_argument("--cache-dir", type=Path, default=Path(".alt-training/cache"))
    parser.add_argument("--template-kwargs", default="{}")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        receipt = qualify(args)
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            with args.output.open("x", encoding="utf-8") as stream:
                json.dump(receipt, stream, indent=2)
                stream.write("\n")
        print(json.dumps(receipt, indent=2))
        return 0 if receipt["status"].startswith("passed_") else 1
    except (DataError, OSError, ValueError, ImportError, importlib.metadata.PackageNotFoundError) as error:
        print(f"Tokenizer qualification failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
