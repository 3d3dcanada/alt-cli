#!/usr/bin/env python3
"""Opt-in, single-GPU QLoRA SFT starter. GPU training has not run in this cloud.

Use --tokenize-only first: it loads a tokenizer, never model weights. The recipe
must be qualified for the chosen architecture, CUDA stack and native tool format.
"""
import argparse
import importlib.metadata
import json
import re
import sys
from pathlib import Path

from features import encode_targets, padded_batch
from prepare import (
    DataError, REVIEW_FLAGS, canonical, load_jsonl, require,
    sha256_file, strict_loads, validate_messages,
)

REFERENCE_PACKAGES = {
    "torch": "2.8.0", "transformers": "4.57.1", "peft": "0.17.1",
    "accelerate": "1.10.1", "bitsandbytes": "0.48.1", "safetensors": "0.6.2",
    "jinja2": "3.1.6",
}


def load_config(path):
    config = strict_loads(Path(path).read_text(encoding="utf-8"))
    require(config.get("schema_version") == 1, "unsupported SFT configuration")
    model, training = config.get("model", {}), config.get("training", {})
    repository = model.get("repository", "")
    require(isinstance(repository, str) and re.fullmatch(r"[\w.-]+/[\w.-]+", repository), "replace the repository placeholder with an exact Hugging Face checkpoint")
    require(isinstance(model.get("revision"), str) and re.fullmatch(r"[0-9a-f]{40}", model["revision"]), "pin the exact 40-character HF training checkpoint commit")
    require(model.get("uncensored") is True, "explicitly select an uncensored/abliterated training checkpoint")
    require(model.get("license_reviewed") is True and isinstance(model.get("license"), str) and bool(model["license"].strip()) and "REPLACE" not in model["license"].upper(), "review derivative/upstream model and output rights first")
    require(training.get("method") == "qlora-nf4", "this starter supports only the qualified NF4 QLoRA recipe")
    require(training.get("compute_dtype") in ("fp16", "bf16"), "choose fp16 or supported bf16 explicitly")
    for key in ("max_length", "max_steps", "gradient_accumulation_steps", "lora_rank", "lora_alpha", "eval_steps", "min_train_task_groups", "min_validation_task_groups"):
        require(type(training.get(key)) is int and training[key] > 0, f"{key} must be a positive integer")
    require(training["max_length"] >= 256, "training context is too short")
    require(training["max_steps"] >= training["eval_steps"], "pilot must reach at least one validation interval")
    require(type(training.get("seed")) is int, "declare a reproducibility seed")
    require(isinstance(training.get("learning_rate"), (int, float)) and 0 < training["learning_rate"] < 1, "invalid learning rate")
    require(isinstance(config.get("chat_template_kwargs", {}), dict), "template controls must be an object")
    return config


def load_prepared(root, config):
    root = Path(root)
    manifest = strict_loads((root / "manifest.json").read_text(encoding="utf-8"))
    require(manifest.get("schema_version") == 1 and manifest.get("purpose") == "reviewed_training_data", "refusing fixture-only or unprepared training data")
    partitions, ids, signatures = {}, set(), set()
    for split in ("train", "validation"):
        name = f"{split}.jsonl"
        require(sha256_file(root / name) == manifest.get("files", {}).get(name), f"prepared {split} data changed")
        rows = load_jsonl(root / name)
        require(bool(rows), f"empty {split} data")
        for row in rows:
            require(row.get("schema_version") == 1 and row.get("split") == split and row.get("synthetic") is False, "invalid production partition")
            review = row.get("review", {})
            require(review.get("status") == "approved" and all(review.get(key) is True for key in REVIEW_FLAGS), "prepared row lacks review")
            require(row.get("actor", {}).get("kind") != "fixture", "fixture in production input")
            if row.get("actor", {}).get("kind") == "model":
                require(row["actor"].get("uncensored") is True, "unapproved model actor")
            validate_messages(row.get("messages"), row.get("tools"))
            require(row.get("id") not in ids, "duplicate prepared ID")
            signature = canonical({"messages": row["messages"], "tools": row["tools"]})
            require(signature not in signatures, "duplicate prepared conversation across partitions")
            ids.add(row["id"])
            signatures.add(signature)
        groups = sorted({row["task_group"] for row in rows})
        require(groups == manifest.get("task_groups", {}).get(split), "prepared task-family manifest changed")
        require(len(rows) == manifest.get("counts", {}).get(split), "prepared count mismatch")
        require(len(groups) >= config["training"][f"min_{split}_task_groups"], f"not enough independent {split} task families for the selected pilot")
        partitions[split] = rows
    require(not set(manifest["task_groups"]["train"]).intersection(manifest["task_groups"]["validation"]), "train/validation family leakage")
    return partitions, manifest


def installed_versions():
    result = {}
    for name in REFERENCE_PACKAGES:
        try:
            result[name] = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError:
            result[name] = None
    return result


def tokenize_dataset(config, partitions, tokenizer):
    encoded, details = {}, {}
    for split, records in partitions.items():
        rows = []
        for record in records:
            rows.extend(encode_targets(tokenizer, record, config["training"]["max_length"], config.get("chat_template_kwargs")))
        encoded[split] = rows
        details[split] = {
            "trajectories": len(records), "assistant_targets": len(rows),
            "total_input_tokens": sum(len(row["input_ids"]) for row in rows),
            "supervised_tokens": sum(sum(label != -100 for label in row["labels"]) for row in rows),
            "maximum_sequence_tokens": max(len(row["input_ids"]) for row in rows),
        }
    return encoded, details


def validate_resume(directory,receipt_path,config,dataset_sha):
    require(receipt_path is not None,"Resume requires the prior receipt")
    previous=strict_loads(Path(receipt_path).read_text())
    require(previous.get("config")==config and previous.get("dataset_manifest_sha256")==dataset_sha,"Resume config or dataset changed")
    manifest=previous.get("checkpoint_manifests",{}).get(str(Path(directory).resolve()))
    require(isinstance(manifest,dict) and bool(manifest),"Checkpoint was not recorded by this training run")
    require({str(p.relative_to(directory)):sha256_file(p) for p in Path(directory).rglob('*') if p.is_file()}==manifest,"Checkpoint files changed")
    return previous


def run(args):
    config = load_config(args.config)
    partitions, manifest = load_prepared(args.dataset, config)
    resume=getattr(args,"resume_from",None)
    if resume:
        require(args.train,"Checkpoint resume is a training operation")
        validate_resume(resume,args.resume_receipt,config,sha256_file(args.dataset / "manifest.json"))
    else:require(getattr(args,"resume_receipt",None) is None,"A resume receipt needs its checkpoint directory")
    versions = installed_versions()
    if args.train:
        require(versions == REFERENCE_PACKAGES, "install the reference pins or explicitly revise/qualify the recipe; installed versions differ")
    else:
        require(versions["transformers"] == REFERENCE_PACKAGES["transformers"], "tokenization qualification requires the pinned Transformers package")
    from transformers import AutoConfig, AutoTokenizer
    model_info = config["model"]
    architecture = AutoConfig.from_pretrained(
        model_info["repository"], revision=model_info["revision"],
        trust_remote_code=False, cache_dir=str(args.cache_dir),
    )
    if architecture.architectures:
        require(any(name.endswith("ForCausalLM") for name in architecture.architectures), "this starter requires a supported text-only causal-LM architecture; qualify a separate loader for conditional/multimodal models")
    tokenizer = AutoTokenizer.from_pretrained(
        model_info["repository"], revision=model_info["revision"],
        trust_remote_code=False, use_fast=True, cache_dir=str(args.cache_dir),
    )
    require(bool(tokenizer.chat_template), "checkpoint has no native chat template; qualify it before training")
    if tokenizer.pad_token_id is None:
        require(tokenizer.eos_token_id is not None, "tokenizer has neither padding nor EOS token")
        tokenizer.pad_token = tokenizer.eos_token
    encoded, details = tokenize_dataset(config, partitions, tokenizer)
    import hashlib
    templates = sorted({
        tokenizer.get_chat_template(tools=record["tools"] or None)
        for records in partitions.values() for record in records
    })
    receipt = {
        "schema_version": 1, "config": config, "packages": versions,
        "dataset_manifest_sha256": sha256_file(args.dataset / "manifest.json"),
        "dataset_files": manifest["files"],
        "chat_template_sha256": [hashlib.sha256(template.encode()).hexdigest() for template in templates],
        "encoding": details, "training_run_performed": False,
        "status": "tokenization_qualified",
    }
    if args.tokenize_only:
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            with args.output.open("x", encoding="utf-8") as stream:
                json.dump(receipt, stream, indent=2)
                stream.write("\n")
        print(json.dumps(receipt, indent=2))
        return
    require(args.output is not None, "--train requires a new --output directory")
    import torch
    from peft import LoraConfig, get_peft_model, prepare_model_for_kbit_training
    from transformers import AutoModelForCausalLM, BitsAndBytesConfig, Trainer, TrainerCallback, TrainingArguments, set_seed
    require(torch.cuda.is_available(), "no usable CUDA GPU; useful 7B/9B training is not qualified on this CPU environment")
    require(torch.cuda.device_count() == 1, "select one GPU with CUDA_VISIBLE_DEVICES; this starter is not a distributed recipe")
    train = config["training"]
    if train["compute_dtype"] == "bf16":
        require(torch.cuda.is_bf16_supported(), "selected GPU does not support the configured BF16 compute")
    dtype = torch.bfloat16 if train["compute_dtype"] == "bf16" else torch.float16
    require(not args.output.exists(), "output exists; keep it for evidence and use a new directory")
    args.output.mkdir(parents=True, exist_ok=False)
    receipt.update({
        "status": "starting", "cuda_runtime": torch.version.cuda,
        "gpu_name": torch.cuda.get_device_name(0),
        "gpu_capability": list(torch.cuda.get_device_capability(0)),
        "free_vram_before_load_bytes": torch.cuda.mem_get_info(0)[0],
        "note": "Training success/loss does not establish held-out task quality or GTX 1070 deployment fit.",
    })
    receipt_path = args.output / "run-receipt.json"

    def save_receipt():
        receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")

    save_receipt()
    set_seed(train["seed"])
    try:
        model = AutoModelForCausalLM.from_pretrained(
            model_info["repository"], revision=model_info["revision"],
            trust_remote_code=False, use_safetensors=True,
            torch_dtype=dtype, device_map={"": 0}, attn_implementation="eager",
            cache_dir=str(args.cache_dir),
            quantization_config=BitsAndBytesConfig(
                load_in_4bit=True, bnb_4bit_quant_type="nf4",
                bnb_4bit_use_double_quant=True, bnb_4bit_compute_dtype=dtype,
            ),
        )
        model.config.use_cache = False
        model = prepare_model_for_kbit_training(
            model, use_gradient_checkpointing=True,
            gradient_checkpointing_kwargs={"use_reentrant": False},
        )
        model = get_peft_model(model, LoraConfig(
            task_type="CAUSAL_LM", r=train["lora_rank"], lora_alpha=train["lora_alpha"],
            lora_dropout=0.05, target_modules="all-linear", bias="none",
        ))
        receipt["trainable_parameters"] = model.get_nb_trainable_parameters()[0]
        receipt["status"] = "training"
        receipt["training_run_performed"] = True
        save_receipt()

        def collate(rows):
            return {key: torch.tensor(values, dtype=torch.long) for key, values in padded_batch(rows, tokenizer.pad_token_id).items()}

        class CheckpointReceipt(TrainerCallback):
            def on_save(self, args, state, control, **kwargs):
                directory=Path(args.output_dir)/f"checkpoint-{state.global_step}"
                if directory.is_dir():
                    receipt.setdefault("checkpoint_manifests", {})[str(directory.resolve())]={str(p.relative_to(directory)):sha256_file(p) for p in directory.rglob('*') if p.is_file()}
                    save_receipt()
                return control

        trainer = Trainer(
            callbacks=[CheckpointReceipt()],
            model=model, processing_class=tokenizer, data_collator=collate,
            train_dataset=encoded["train"], eval_dataset=encoded["validation"],
            args=TrainingArguments(
                output_dir=str(args.output / "checkpoints"),
                max_steps=train["max_steps"], learning_rate=train["learning_rate"],
                per_device_train_batch_size=1, per_device_eval_batch_size=1,
                gradient_accumulation_steps=train["gradient_accumulation_steps"],
                gradient_checkpointing=True, gradient_checkpointing_kwargs={"use_reentrant": False},
                fp16=train["compute_dtype"] == "fp16", bf16=train["compute_dtype"] == "bf16",
                optim="adamw_torch", warmup_ratio=0.05,
                eval_strategy="steps", eval_steps=train["eval_steps"],
                save_strategy="steps", save_steps=train["eval_steps"], save_total_limit=2,
                load_best_model_at_end=True, metric_for_best_model="eval_loss",
                greater_is_better=False, logging_steps=5, report_to="none",
                seed=train["seed"], data_seed=train["seed"], push_to_hub=False,
            ),
        )
        resume=getattr(args,"resume_from",None)
        if resume:
            validate_resume(resume,args.resume_receipt,config,receipt["dataset_manifest_sha256"])
            receipt["resume_from"] = str(resume.resolve());save_receipt()
        result = trainer.train(resume_from_checkpoint=str(resume.resolve()) if resume else None)
        trainer.save_model(str(args.output / "adapter"))
        tokenizer.save_pretrained(str(args.output / "adapter"))
        receipt.update({
            "status": "training_finished_unqualified", "train_metrics": result.metrics,
            "validation_metrics": trainer.evaluate(),
            "peak_cuda_allocated_bytes": torch.cuda.max_memory_allocated(0),
            "peak_cuda_reserved_bytes": torch.cuda.max_memory_reserved(0),
            "adapter_files": {p.name: sha256_file(p) for p in (args.output / "adapter").iterdir() if p.is_file()},
        })
    except (Exception, KeyboardInterrupt) as error:
        receipt.update({"status": "interrupted" if isinstance(error, KeyboardInterrupt) else "failed", "error_type": type(error).__name__})
        save_receipt()
        raise
    save_receipt()
    print(json.dumps(receipt, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--dataset", type=Path, required=True)
    parser.add_argument("--cache-dir", type=Path, default=Path(".alt-training/cache"))
    parser.add_argument("--resume-from",type=Path,help="Exact checkpoint directory from a recorded prior run")
    parser.add_argument("--resume-receipt",type=Path,help="Prior run receipt with immutable checkpoint file hashes")
    parser.add_argument("--output", type=Path, help="New receipt file for tokenization; new run directory for training")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--tokenize-only", action="store_true")
    mode.add_argument("--train", action="store_true")
    args = parser.parse_args()
    try:
        run(args)
    except (DataError, OSError, ValueError, ImportError) as error:
        print(f"SFT preparation/training failed: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
