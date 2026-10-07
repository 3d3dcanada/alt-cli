#!/usr/bin/env python3
"""Validate reviewed Alt trajectories and export task-family-separated SFT data.

Standard library only. This checks recorded evidence, not software correctness
itself; reviewers must run the independent checks and review their coverage.
"""
import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

SHA256 = re.compile(r"[0-9a-f]{64}\Z")
REVISION = re.compile(r"(?:[0-9a-f]{40}|[0-9a-f]{64})\Z")
REVIEW_FLAGS = (
    "source_correctness", "tool_results_real", "rights_reviewed",
    "privacy_reviewed", "split_reviewed",
)


class DataError(ValueError):
    pass


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False)


def strict_loads(value):
    def constant(name):
        raise DataError(f"non-JSON numeric constant: {name}")

    def unique_object(pairs):
        result = {}
        for key, item in pairs:
            require(key not in result, f"duplicate JSON key: {key}")
            result[key] = item
        return result

    return json.loads(value, parse_constant=constant, object_pairs_hook=unique_object)


def sha256_file(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def require(condition, message):
    if not condition:
        raise DataError(message)


def text_field(value, label):
    require(isinstance(value, str) and bool(value.strip()), f"{label} must be nonempty text")


def verified_file(root, relative, digest):
    text_field(relative, "evidence path")
    require(isinstance(digest, str) and SHA256.fullmatch(digest), "invalid evidence SHA256")
    path = Path(relative)
    require(not path.is_absolute() and ".." not in path.parts, "evidence path must stay under evidence root")
    resolved = (root / path).resolve()
    require(resolved.is_relative_to(root.resolve()), "evidence symlink leaves evidence root")
    require(resolved.is_file(), f"missing evidence: {relative}")
    require(sha256_file(resolved) == digest, f"evidence hash mismatch: {relative}")
    return resolved


def validate_arguments(value, schema, root=None, depth=0):
    """Validate native arguments before they become assistant training targets.

    Supports the bounded JSON-schema subset used by Alt and common function tools;
    external references and unimplemented assertion keywords need explicit review.
    """
    require(depth < 32 and isinstance(schema, (dict, bool)), "invalid or recursive tool schema")
    if isinstance(schema, bool):
        require(schema, "arguments forbidden by schema")
        return
    root = schema if root is None else root
    supported = {"type", "properties", "required", "additionalProperties", "items", "enum", "const", "minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "minLength", "maxLength", "pattern", "minItems", "maxItems", "uniqueItems", "anyOf", "oneOf", "allOf", "$ref", "$defs", "definitions", "$schema", "description", "title", "default", "examples", "format", "nullable"}
    require(not set(schema) - supported, "tool schema has unqualified assertion keywords")
    if "$ref" in schema:
        ref = schema["$ref"]
        require(isinstance(ref, str) and ref.startswith("#/"), "external schema references are not qualified")
        resolved = root
        for part in ref[2:].split("/"):
            require(isinstance(resolved, dict) and part.replace("~1", "/").replace("~0", "~") in resolved, "unresolved tool schema reference")
            resolved = resolved[part.replace("~1", "/").replace("~0", "~")]
        validate_arguments(value, resolved, root, depth + 1)
    for keyword in ("anyOf", "oneOf", "allOf"):
        if keyword in schema:
            variants = schema[keyword]
            require(isinstance(variants, list) and variants, "empty tool schema alternatives")
            matches = 0
            for variant in variants:
                try: validate_arguments(value, variant, root, depth + 1); matches += 1
                except DataError: pass
            require(matches >= 1 if keyword == "anyOf" else matches == 1 if keyword == "oneOf" else matches == len(variants), "native arguments violate schema alternatives")
    kind = schema.get("type")
    kinds = kind if isinstance(kind, list) else [kind] if kind else []
    types = {"object": isinstance(value, dict), "array": isinstance(value, list), "string": isinstance(value, str), "integer": type(value) is int, "number": type(value) in (int, float), "boolean": type(value) is bool, "null": value is None}
    require(not kinds or any(types.get(k, False) for k in kinds) or (value is None and schema.get("nullable") is True), "native argument type mismatch")
    for keyword in ("enum", "const"):
        if keyword in schema:
            allowed = schema[keyword] if keyword == "enum" else [schema[keyword]]
            require(isinstance(allowed, list) and any(canonical(value) == canonical(item) for item in allowed), "native argument outside declared values")
    if isinstance(value, dict):
        props = schema.get("properties", {})
        require(isinstance(props, dict) and all(k in value for k in schema.get("required", [])), "missing required tool arguments")
        for key, item in value.items():
            if key in props: validate_arguments(item, props[key], root, depth + 1)
            elif "additionalProperties" in schema: validate_arguments(item, schema["additionalProperties"], root, depth + 1)
    elif isinstance(value, list):
        require(len(value) >= schema.get("minItems", 0) and len(value) <= schema.get("maxItems", len(value)), "native argument list length mismatch")
        if schema.get("uniqueItems"): require(len({canonical(item) for item in value}) == len(value), "native argument list has duplicates")
        for item in value: validate_arguments(item, schema.get("items", True), root, depth + 1)
    elif isinstance(value, str):
        require(len(value) >= schema.get("minLength", 0) and len(value) <= schema.get("maxLength", len(value)), "native argument text length mismatch")
        if "pattern" in schema: require(re.search(schema["pattern"], value) is not None, "native argument violates pattern")
    elif type(value) in (int, float):
        import math
        require(type(value) is int or math.isfinite(value), "nonfinite native argument")
        for key, predicate in (("minimum", lambda bound: value >= bound), ("maximum", lambda bound: value <= bound), ("exclusiveMinimum", lambda bound: value > bound), ("exclusiveMaximum", lambda bound: value < bound)):
            if key in schema: require(predicate(schema[key]), f"native argument violates {key}")


def validate_messages(messages, tools):
    require(isinstance(tools, list), "tools must be a list")
    names = {}
    for tool in tools:
        require(isinstance(tool, dict) and tool.get("type") == "function", "use native function tool definitions")
        function = tool.get("function", {})
        require(isinstance(function, dict), "invalid tool definition")
        name = function.get("name")
        text_field(name, "tool name")
        require(name not in names, "duplicate tool definition")
        params = function.get("parameters")
        require(isinstance(params, dict) and params.get("type") == "object", "tool parameters must declare an object")
        require(isinstance(params.get("required", []), list), "required parameters must be a list")
        names[name] = params
    require(isinstance(messages, list) and len(messages) >= 2, "need a complete conversation")
    pending, seen = {}, set()
    user_seen = False
    for index, message in enumerate(messages):
        require(isinstance(message, dict), "message must be an object")
        role = message.get("role")
        require(role in ("system", "user", "assistant", "tool"), "unsupported message role")
        content = message.get("content")
        require(isinstance(content, str) or (role == "assistant" and content is None), "use text content; preserve raw multimodal evidence separately")
        if "reasoning_content" in message:
            require(role == "assistant" and isinstance(message["reasoning_content"], str), "invalid recorded reasoning field")
        if role == "system":
            require(index == 0, "system instructions must be first")
        elif role == "user":
            require(not pending, "user message interrupts unresolved tool calls")
            user_seen = True
        elif role == "assistant":
            require(user_seen, "assistant target has no user request")
            require(not pending, "assistant message precedes required tool results")
            calls = message.get("tool_calls", [])
            require(isinstance(calls, list), "tool_calls must be a list")
            require(bool(content) or bool(calls), "empty assistant target")
            for call in calls:
                require(isinstance(call, dict) and call.get("type") == "function", "invalid native tool call")
                call_id = call.get("id")
                text_field(call_id, "tool call ID")
                require(call_id not in seen, "reused tool call ID")
                function = call.get("function", {})
                require(isinstance(function, dict), "invalid tool function")
                name, arguments = function.get("name"), function.get("arguments")
                require(isinstance(name, str) and name in names, "call to an undeclared tool")
                require(isinstance(arguments, dict), "normalize native arguments to an object, not a JSON string")
                validate_arguments(arguments, names[name])
                seen.add(call_id)
                pending[call_id] = name
        else:
            call_id = message.get("tool_call_id")
            require(isinstance(call_id, str) and call_id in pending, "orphan or duplicate tool result")
            pending.pop(call_id)
        require(not message.get("tool_calls") or role == "assistant", "only assistants request tools")
    require(not pending, "unfinished tool exchange")
    require(messages[-1].get("role") == "assistant" and bool(messages[-1].get("content")), "need a recorded final assistant response")


def validate_record(record, registry, root, allow_fixtures=False):
    require(isinstance(record, dict) and record.get("schema_version") == 1, "unsupported trajectory schema")
    for field in ("id", "task_group", "split"):
        text_field(record.get(field), field)
    group, split = record["task_group"], record["split"]
    require(split in ("train", "validation"), "sealed/test trajectories are not training input")
    require(registry.get(group) == split, "task family disagrees with frozen split registry")
    require(record.get("response_style") in ("concise", "mixed", "long"), "unknown response style")
    synthetic = record.get("synthetic")
    require(type(synthetic) is bool, "declare whether the trace is synthetic")
    require(not synthetic or allow_fixtures, "synthetic test fixtures cannot enter production data")
    actor = record.get("actor", {})
    require(isinstance(actor, dict), "invalid actor")
    require(actor.get("kind") in ("human", "model", "fixture"), "declare actor kind")
    require(actor.get("kind") != "fixture" or (synthetic and allow_fixtures), "fixture actor requires fixture-only export")
    if actor.get("kind") == "model":
        text_field(actor.get("repository"), "actor repository")
        require(isinstance(actor.get("revision"), str) and REVISION.fullmatch(actor["revision"]), "pin actor revision")
        require(actor.get("uncensored") is True, "live model actor must be explicitly uncensored/abliterated")
        require(isinstance(actor.get("artifact_sha256"), str) and SHA256.fullmatch(actor["artifact_sha256"]), "identify the actor weight artifact or weight-manifest SHA256")
        require(isinstance(actor.get("template_sha256"), str) and SHA256.fullmatch(actor["template_sha256"]), "identify the actor native template SHA256")
        text_field(actor.get("quantization"), "actor quantization/precision")
        text_field(actor.get("runtime_build"), "actor runtime build")
    review = record.get("review", {})
    require(isinstance(review, dict) and review.get("status") == "approved", "trajectory requires approval after correctness review")
    text_field(review.get("reviewer"), "reviewer")
    require(all(review.get(flag) is True for flag in REVIEW_FLAGS), "incomplete correctness, evidence, rights, privacy or split review")
    validate_messages(record.get("messages"), record.get("tools"))
    source = record.get("source", {})
    require(isinstance(source, dict), "invalid source provenance")
    for field in ("repository", "license"):
        text_field(source.get(field), f"source {field}")
    require(source["license"].lower() not in ("unknown", "unreviewed", "pending", "none"), "source rights remain unresolved")
    require(isinstance(source.get("revision"), str) and REVISION.fullmatch(source["revision"]), "pin source repository revision")
    verified_file(root, source.get("raw_trace_path"), source.get("raw_trace_sha256"))
    normalized_path = verified_file(root, source.get("transcript_path"), source.get("transcript_sha256"))
    transcript = strict_loads(normalized_path.read_text(encoding="utf-8"))
    require(transcript.get("messages") == record["messages"] and transcript.get("tools") == record["tools"], "messages/tools differ from reviewed transcript evidence")
    verified_file(root, source.get("snapshot_path"), source.get("snapshot_sha256"))
    checks = record.get("checks")
    require(isinstance(checks, list) and bool(checks), "independent behavioral evidence is required")
    check_names = set()
    for check in checks:
        require(isinstance(check, dict), "invalid check reference")
        text_field(check.get("name"), "check name")
        require(check["name"] not in check_names, "duplicate check name")
        check_names.add(check["name"])
        path = verified_file(root, check.get("path"), check.get("sha256"))
        observation = strict_loads(path.read_text(encoding="utf-8"))
        require(observation.get("schema_version") == 1 and observation.get("name") == check["name"], "check identity mismatch")
        require(observation.get("kind") == "independent_behavior", "format/build/self-report alone is not a behavioral oracle")
        require(observation.get("passed") is True and type(observation.get("exit_code")) is int and observation["exit_code"] == 0, "behavioral evidence did not pass")
        require(type(observation.get("tests_run")) is int and observation["tests_run"] > 0, "behavioral check ran no assertions")
        require(observation.get("source_sha256") == source["snapshot_sha256"], "behavioral evidence belongs to a different source snapshot")
        verified_file(root, observation.get("raw_output_path"), observation.get("raw_output_sha256"))
    return record


def load_jsonl(path):
    rows = []
    with Path(path).open(encoding="utf-8") as stream:
        for number, line in enumerate(stream, 1):
            if not line.strip():
                continue
            try:
                rows.append(strict_loads(line))
            except json.JSONDecodeError as error:
                raise DataError(f"{Path(path).name}:{number}: invalid JSON") from error
    return rows


def prepare(records_path, registry_path, evidence_root, output, allow_fixtures=False):
    registry_doc = strict_loads(Path(registry_path).read_text(encoding="utf-8"))
    require(registry_doc.get("schema_version") == 1, "unsupported split registry")
    registry = registry_doc.get("families")
    require(isinstance(registry, dict) and bool(registry), "freeze a nonempty task-family registry before collection")
    require(all(isinstance(k, str) and k and v in ("train", "validation", "test") for k, v in registry.items()), "invalid task-family partition")
    rows = load_jsonl(records_path)
    require(bool(rows), "no training records")
    ids, signatures = set(), set()
    for row in rows:
        try:
            validate_record(row, registry, Path(evidence_root), allow_fixtures)
            require(row["id"] not in ids, "duplicate trajectory ID")
            signature = hashlib.sha256(canonical({"messages": row["messages"], "tools": row["tools"]}).encode()).hexdigest()
            require(signature not in signatures, "duplicate conversation; repeated attempts are not new tasks")
            ids.add(row["id"])
            signatures.add(signature)
        except (DataError, KeyError, TypeError, AttributeError, json.JSONDecodeError) as error:
            label = row.get("id", "unidentified") if isinstance(row, dict) else "unidentified"
            raise DataError(f"{label}: {error}") from error
    partitions = {split: sorted([r for r in rows if r["split"] == split], key=lambda r: r["id"]) for split in ("train", "validation")}
    require(all(partitions.values()), "need nonempty train and validation partitions; never duplicate a task to fill one")
    manifest = {
        "schema_version": 1,
        "purpose": "fixture_only" if allow_fixtures else "reviewed_training_data",
        "records_sha256": sha256_file(records_path),
        "split_registry_sha256": sha256_file(registry_path),
        "files": {}, "counts": {}, "task_groups": {},
        "review_boundary": "Hashes and attestations checked; independent check quality, privacy and family grouping require real review.",
    }
    output = Path(output)
    output.mkdir(parents=True, exist_ok=False)
    for split, partition in partitions.items():
        path = output / f"{split}.jsonl"
        with path.open("x", encoding="utf-8") as stream:
            for row in partition:
                stream.write(canonical(row) + "\n")
        manifest["files"][path.name] = sha256_file(path)
        manifest["counts"][split] = len(partition)
        manifest["task_groups"][split] = sorted({r["task_group"] for r in partition})
    # Written last: a partially written directory is not a prepared dataset.
    with (output / "manifest.json").open("x", encoding="utf-8") as stream:
        json.dump(manifest, stream, ensure_ascii=False, indent=2)
        stream.write("\n")
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--records", type=Path, required=True)
    parser.add_argument("--split-registry", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--allow-fixtures", action="store_true", help="Mark entire export fixture-only; SFT driver refuses it")
    args = parser.parse_args()
    try:
        result = prepare(args.records, args.split_registry, args.evidence_root, args.output, args.allow_fixtures)
    except (DataError, OSError, ValueError) as error:
        print(f"Data preparation failed: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
