import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from features import encode_targets, padded_batch
from prepare import DataError, canonical, prepare, sha256_file, strict_loads, validate_record
from train_sft import load_config, load_prepared
from qualify_tokenizer import fixture as tokenizer_fixture


class Fixture:
    def __init__(self, root):
        self.root = root
        self.registry = {"repair-a": "train", "repair-b": "validation", "unseen-c": "test"}

    def write(self, name, value):
        path = self.root / name
        path.write_text(canonical(value) + "\n", encoding="utf-8")
        return name, sha256_file(path)

    def record(self, label="a", split="train"):
        tools = [{"type": "function", "function": {
            "name": "read", "description": "Read a file", "parameters": {
                "type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"],
            },
        }}]
        messages = [
            {"role": "system", "content": "Use actual evidence."},
            {"role": "user", "content": f"Inspect fixture {label}."},
            {"role": "assistant", "content": None, "tool_calls": [{
                "id": "call-1", "type": "function", "function": {"name": "read", "arguments": {"path": f"{label}.py"}},
            }]},
            {"role": "tool", "tool_call_id": "call-1", "content": f"fixture {label}: return 1"},
            {"role": "assistant", "content": "The recorded fixture check passed."},
        ]
        raw_path, raw_hash = self.write(f"{label}-raw.json", {"fixture": True, "events": messages})
        transcript_path, transcript_hash = self.write(f"{label}-transcript.json", {"messages": messages, "tools": tools})
        snapshot_path, snapshot_hash = self.write(f"{label}-source.json", {"fixture_source": f"def {label}(): return 1"})
        output_path, output_hash = self.write(f"{label}-output.json", {"fixture": True, "assertions": 2})
        check_path, check_hash = self.write(f"{label}-check.json", {
            "schema_version": 1, "name": "fixture-behavior", "kind": "independent_behavior",
            "passed": True, "exit_code": 0, "tests_run": 2, "source_sha256": snapshot_hash,
            "raw_output_path": output_path, "raw_output_sha256": output_hash,
        })
        return {
            "schema_version": 1, "id": f"fixture-{label}", "task_group": f"repair-{label}",
            "split": split, "response_style": "concise", "synthetic": True,
            "actor": {"kind": "fixture"}, "messages": messages, "tools": tools,
            "source": {"repository": "fixture://local", "revision": "a" * 40, "license": "Apache-2.0",
                       "raw_trace_path": raw_path, "raw_trace_sha256": raw_hash,
                       "transcript_path": transcript_path, "transcript_sha256": transcript_hash,
                       "snapshot_path": snapshot_path, "snapshot_sha256": snapshot_hash},
            "checks": [{"name": "fixture-behavior", "path": check_path, "sha256": check_hash}],
            "review": {"status": "approved", "reviewer": "unit-test fixture",
                       "source_correctness": True, "tool_results_real": True,
                       "rights_reviewed": True, "privacy_reviewed": True, "split_reviewed": True},
        }

    def refresh_transcript(self, record):
        name, digest = self.write(record["source"]["transcript_path"], {"messages": record["messages"], "tools": record["tools"]})
        record["source"]["transcript_sha256"] = digest

    def change_check(self, record, key, value):
        path = self.root / record["checks"][0]["path"]
        observation = json.loads(path.read_text())
        observation[key] = value
        path.write_text(canonical(observation))
        record["checks"][0]["sha256"] = sha256_file(path)


class NativeFixtureTokenizer:
    """Weight-free deterministic serializer; it is not model capability evidence."""
    def apply_chat_template(self, messages, tools=None, tokenize=True, add_generation_prompt=False, **kwargs):
        text = ""
        if tools:
            text += "tools:" + canonical(tools) + "\n"
        for message in messages:
            text += message["role"] + ":" + (message.get("content") or "")
            if message.get("tool_calls"):
                text += canonical(message["tool_calls"])
            if message.get("reasoning_content"):
                text += message["reasoning_content"]
            text += "<end>\n"
        if add_generation_prompt:
            text += "assistant:"
        return list(text.encode())


class DataTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.fx = Fixture(self.root)
        self.row = self.fx.record()

    def tearDown(self):
        self.temp.cleanup()

    def validate(self, row=None, allow=True):
        return validate_record(row or self.row, self.fx.registry, self.root, allow)

    def rejected(self, fragment, row=None, allow=True):
        with self.assertRaisesRegex(DataError, fragment):
            self.validate(row, allow)

    def prepared(self, rows=None, output="prepared", allow=True):
        records = self.root / "records.jsonl"
        records.write_text("".join(canonical(r) + "\n" for r in (rows or [self.row, self.fx.record("b", "validation")])))
        registry = self.root / "split-registry.json"
        registry.write_text(canonical({"schema_version": 1, "families": self.fx.registry}))
        return prepare(records, registry, self.root, self.root / output, allow)

    def test_valid_reviewed_fixture(self):
        self.assertEqual(self.validate()["id"], "fixture-a")

    def test_fixture_rejected_by_production_default(self):
        self.rejected("synthetic", allow=False)

    def test_unreviewed_source_rejected(self):
        self.row["review"]["source_correctness"] = False
        self.rejected("incomplete")

    def test_privacy_rights_and_split_attestations_required(self):
        for flag in ("privacy_reviewed", "rights_reviewed", "split_reviewed"):
            row = copy.deepcopy(self.row)
            row["review"][flag] = False
            self.rejected("incomplete", row)

    def test_test_family_never_exported(self):
        self.row.update(task_group="unseen-c", split="test")
        self.rejected("sealed/test")

    def test_family_split_mismatch(self):
        self.row["split"] = "validation"
        self.rejected("family disagrees")

    def test_censored_actor_rejected(self):
        self.row["actor"] = {"kind": "model", "repository": "fixture/model", "revision": "b" * 40, "uncensored": False}
        self.rejected("explicitly uncensored")

    def test_model_actor_must_identify_exact_artifact_and_template(self):
        self.row["actor"] = {"kind": "model", "repository": "fixture/model", "revision": "b" * 40, "uncensored": True}
        self.rejected("weight artifact")
        self.row["actor"].update(artifact_sha256="c" * 64, template_sha256="d" * 64, quantization="Q4_K_M", runtime_build="fixture-runtime-1")
        self.validate()

    def test_unresolved_source_license_rejected(self):
        self.row["source"]["license"] = "unknown"
        self.rejected("rights remain unresolved")

    def test_trace_hash_mismatch(self):
        (self.root / self.row["source"]["raw_trace_path"]).write_text("changed")
        self.rejected("hash mismatch")

    def test_fabricated_transcript_rejected(self):
        self.row["messages"][-1]["content"] = "Invented successful result"
        self.rejected("differ from reviewed")

    def test_orphan_tool_result_rejected(self):
        self.row["messages"][3]["tool_call_id"] = "not-called"
        self.rejected("orphan")

    def test_unfinished_tool_exchange_rejected(self):
        del self.row["messages"][3]
        self.rejected("precedes required")

    def test_unknown_tool_rejected(self):
        self.row["messages"][2]["tool_calls"][0]["function"]["name"] = "unknown"
        self.rejected("undeclared")

    def test_string_arguments_rejected(self):
        self.row["messages"][2]["tool_calls"][0]["function"]["arguments"] = '{"path":"a.py"}'
        self.rejected("not a JSON string")

    def test_missing_required_argument_rejected(self):
        self.row["messages"][2]["tool_calls"][0]["function"]["arguments"] = {}
        self.rejected("missing required")

    def test_stale_behavioral_check_rejected(self):
        self.fx.change_check(self.row, "source_sha256", "e" * 64)
        self.rejected("different source")

    def test_failed_or_empty_checks_rejected(self):
        for field, value, fragment in [("passed", False, "did not pass"), ("tests_run", 0, "no assertions"), ("kind", "format", "behavioral oracle")]:
            row = self.fx.record()
            self.fx.change_check(row, field, value)
            self.rejected(fragment, row)

    def test_raw_check_output_hash_required(self):
        self.fx.change_check(self.row, "raw_output_sha256", "e" * 64)
        self.rejected("hash mismatch")

    def test_path_traversal_and_symlink_escape_rejected(self):
        self.row["source"]["raw_trace_path"] = "../outside.json"
        self.rejected("stay under")
        with tempfile.TemporaryDirectory() as outside:
            path = Path(outside) / "trace.json"
            path.write_text("outside")
            (self.root / "outside-link").symlink_to(path)
            self.row["source"]["raw_trace_path"] = "outside-link"
            self.rejected("symlink leaves")

    def test_export_counts_and_hashes(self):
        manifest = self.prepared()
        self.assertEqual(manifest["purpose"], "fixture_only")
        self.assertEqual(manifest["task_groups"], {"train": ["repair-a"], "validation": ["repair-b"]})
        self.assertEqual(manifest["files"]["train.jsonl"], sha256_file(self.root / "prepared/train.jsonl"))

    def test_duplicate_attempt_not_new_training_data(self):
        duplicate = copy.deepcopy(self.row)
        duplicate["id"] = "repeated-a"
        with self.assertRaisesRegex(DataError, "duplicate conversation"):
            self.prepared([self.row, duplicate, self.fx.record("b", "validation")])
        self.assertFalse((self.root / "prepared").exists())

    def test_existing_output_never_overwritten(self):
        self.prepared()
        before = (self.root / "prepared/manifest.json").read_bytes()
        with self.assertRaises(FileExistsError):
            self.prepared()
        self.assertEqual(before, (self.root / "prepared/manifest.json").read_bytes())

    def test_fixture_manifest_rejected_by_training_loader(self):
        self.prepared()
        config = {"training": {"min_train_task_groups": 1, "min_validation_task_groups": 1}}
        with self.assertRaisesRegex(DataError, "fixture-only"):
            load_prepared(self.root / "prepared", config)

    def test_cli_exports_fixture_and_training_refuses_it_before_loading(self):
        self.prepared()
        training = Path(__file__).resolve().parents[1]
        result = subprocess.run([
            sys.executable, str(training / "prepare.py"), "--records", str(self.root / "records.jsonl"),
            "--split-registry", str(self.root / "split-registry.json"), "--evidence-root", str(self.root),
            "--output", str(self.root / "cli-export"), "--allow-fixtures",
        ], capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["purpose"], "fixture_only")
        config = json.loads((training / "configs/sft-7b.json").read_text())
        config["model"].update(repository="fixture/student", revision="d" * 40, license="Apache-2.0", license_reviewed=True)
        path = self.root / "qualified-test-config.json"
        path.write_text(canonical(config))
        attempted = subprocess.run([
            sys.executable, str(training / "train_sft.py"), "--config", str(path),
            "--dataset", str(self.root / "cli-export"), "--train", "--output", str(self.root / "never-created"),
        ], capture_output=True, text=True, timeout=10)
        self.assertEqual(attempted.returncode, 2)
        self.assertIn("fixture-only", attempted.stderr)
        self.assertFalse((self.root / "never-created").exists())

    def test_tampered_prepared_input_rejected(self):
        for row in (self.row,):
            row.update(synthetic=False, actor={"kind": "human"})
        second = self.fx.record("b", "validation")
        second.update(synthetic=False, actor={"kind": "human"})
        self.prepared([self.row, second], allow=False)
        config = {"training": {"min_train_task_groups": 1, "min_validation_task_groups": 1}}
        load_prepared(self.root / "prepared", config)
        with (self.root / "prepared/train.jsonl").open("a") as stream:
            stream.write("{}\n")
        with self.assertRaisesRegex(DataError, "data changed"):
            load_prepared(self.root / "prepared", config)


class EncodingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.row = Fixture(Path(self.temp.name)).record()
        self.tokenizer = NativeFixtureTokenizer()

    def tearDown(self):
        self.temp.cleanup()

    def test_only_current_assistant_target_supervised(self):
        targets = encode_targets(self.tokenizer, self.row, 4096)
        self.assertEqual(len(targets), 2)
        for target, index in zip(targets, (2, 4)):
            prompt = self.tokenizer.apply_chat_template(self.row["messages"][:index], tools=self.row["tools"], add_generation_prompt=True)
            self.assertTrue(all(label == -100 for label in target["labels"][:len(prompt)]))
            self.assertEqual(target["labels"][len(prompt):], target["input_ids"][len(prompt):])
        # The real tool observation is context for the final answer, not its target.
        self.assertIn(b"fixture a: return 1", bytes(targets[1]["input_ids"]))
        self.assertNotIn(b"fixture a: return 1", bytes(x for x in targets[1]["labels"] if x != -100))

    def test_padding_stays_masked(self):
        targets = encode_targets(self.tokenizer, self.row, 4096)
        batch = padded_batch(targets, 0)
        width = max(len(row["input_ids"]) for row in targets)
        for index, target in enumerate(targets):
            self.assertEqual(len(batch["labels"][index]), width)
            self.assertTrue(all(x == -100 for x in batch["labels"][index][len(target["input_ids"]):]))
            self.assertTrue(all(x == 0 for x in batch["attention_mask"][index][len(target["input_ids"]):]))

    def test_overlong_trace_never_silently_truncated(self):
        with self.assertRaisesRegex(DataError, "no truncation"):
            encode_targets(self.tokenizer, self.row, 30)

    def test_wrong_generation_prefix_rejected(self):
        class Broken(NativeFixtureTokenizer):
            def apply_chat_template(self, *args, **kwargs):
                result = super().apply_chat_template(*args, **kwargs)
                return result + [999] if kwargs.get("add_generation_prompt") else result
        with self.assertRaisesRegex(DataError, "not an exact completion prefix"):
            encode_targets(Broken(), self.row, 4096)

    def test_template_dropping_tool_arguments_rejected(self):
        class DropsArguments(NativeFixtureTokenizer):
            def apply_chat_template(self, messages, **kwargs):
                changed = copy.deepcopy(messages)
                for message in changed:
                    for call in message.get("tool_calls", []):
                        call["function"]["arguments"] = {}
                return super().apply_chat_template(changed, **kwargs)
        with self.assertRaisesRegex(DataError, "ignores tool arguments"):
            encode_targets(DropsArguments(), self.row, 4096)

    def test_template_controls_cannot_override_encoder(self):
        with self.assertRaisesRegex(DataError, "cannot override"):
            encode_targets(self.tokenizer, self.row, 4096, {"tokenize": False})

    def test_recorded_reasoning_must_serialize(self):
        self.row["messages"][2]["reasoning_content"] = "Read the current file before choosing a repair."
        self.assertEqual(len(encode_targets(self.tokenizer, self.row, 4096)), 2)

    def test_reproducible_tokenizer_fixture_has_two_targets(self):
        record = tokenizer_fixture()
        self.assertTrue(record["synthetic"])
        self.assertEqual(len(encode_targets(self.tokenizer, record, 4096)), 2)


class ConfigTests(unittest.TestCase):
    def test_ambiguous_or_invalid_json_rejected(self):
        for text in ('{"passed":false,"passed":true}', '{"value":NaN}', '{"value":Infinity}'):
            with self.assertRaises(DataError):
                strict_loads(text)

    def test_both_templates_require_exact_model_and_rights(self):
        root = Path(__file__).resolve().parents[1]
        for size in (7, 9):
            with self.assertRaisesRegex(DataError, "repository placeholder"):
                load_config(root / f"configs/sft-{size}b.json")

    def test_config_requires_review_and_validation_interval(self):
        source = Path(__file__).resolve().parents[1] / "configs/sft-7b.json"
        config = json.loads(source.read_text())
        config["model"].update(repository="fixture/student", revision="d" * 40, license="Apache-2.0", license_reviewed=True)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "config.json"
            path.write_text(canonical(config))
            self.assertEqual(load_config(path)["model"]["revision"], "d" * 40)
            config["model"]["license_reviewed"] = False
            path.write_text(canonical(config))
            with self.assertRaisesRegex(DataError, "model and output rights"):
                load_config(path)
            config["model"]["license_reviewed"] = True
            config["training"]["eval_steps"] = 101
            path.write_text(canonical(config))
            with self.assertRaisesRegex(DataError, "validation interval"):
                load_config(path)


if __name__ == "__main__":
    unittest.main()
