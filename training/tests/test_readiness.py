"""Fail-closed readiness without model/GPU downloads."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from prepare import DataError
from readiness import (evaluate, require_training_readiness, verify_prepared_records,
                       collection_receipts, bind_record_to_attempt)
from prepare import sha256_file
from qualify_loader import run as qualify_loader


class ReadinessTests(unittest.TestCase):
    def test_no_corpus_or_gpu_is_a_detailed_blocked_handoff_not_trained(self):
        result=evaluate(hardware={'torch_cuda_available':False})
        self.assertFalse(result['training_ready'])
        self.assertFalse(result['training_run_performed'])
        self.assertEqual(result['approved_families'],{'train':0,'validation':0})
        self.assertTrue(any('32' in reason for reason in result['blockers']))
        self.assertTrue(any('CUDA' in reason for reason in result['blockers']))

    def test_duplicate_attempts_cannot_replace_failed_attempt_accounting(self):
        with tempfile.TemporaryDirectory() as directory:
            ledger=Path(directory)/'ledger.json'
            ledger.write_text(json.dumps({'attempts':[{'id':'a','status':'failed'}, {'id':'a','status':'passed'}]}))
            with self.assertRaises(DataError):evaluate(ledger=ledger,hardware={'torch_cuda_available':False})

    def test_failed_and_unperformed_attempts_are_counted_without_becoming_approved(self):
        with tempfile.TemporaryDirectory() as directory:
            ledger=Path(directory)/'ledger.json'
            ledger.write_text(json.dumps({'attempts':[{'id':'a','status':'failed'}, {'id':'b','status':'unperformed'}, {'id':'c','status':'passed'}]}))
            result=evaluate(ledger=ledger,hardware={'torch_cuda_available':False})
            self.assertEqual(result['attempt_counts'],{'failed':1,'unperformed':1,'passed':1})
            self.assertEqual(result['approved_families']['train'],0)
            self.assertFalse(result['training_ready'])

    def test_training_cannot_start_without_a_bound_readiness_receipt(self):
        with self.assertRaises(DataError):require_training_readiness(None,Path('absent'),Path('absent'))
        with tempfile.TemporaryDirectory() as directory:
            receipt=Path(directory)/'readiness.json';receipt.write_text('{"training_ready":false}')
            with self.assertRaises(DataError):require_training_readiness(receipt,Path('absent'),Path('absent'))

    def test_loader_plan_never_loads_packages_or_claims_a_backward_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            config=json.loads((Path(__file__).resolve().parents[1]/'configs/sft-7b.json').read_text())
            config['model'].update(repository='fixture/no-weights',revision='a'*40,
                                   license='Apache-2.0',license_reviewed=True)
            path=root/'config.json';path.write_text(json.dumps(config))
            with patch('qualify_loader.installed_versions',side_effect=AssertionError('Must not load stack')):
                report=qualify_loader(path,root/'plan')
            self.assertEqual(report['status'],'planned')
            self.assertFalse(report['actual_forward_backward_completed'])
            self.assertFalse(report['training_run_performed'])
            self.assertFalse(report['adapter_saved'])
            with self.assertRaises(DataError):qualify_loader(path,root/'plan')

    def test_wrong_loader_stack_retains_failure_before_loading_any_model(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            config=json.loads((Path(__file__).resolve().parents[1]/'configs/sft-7b.json').read_text())
            config['model'].update(repository='fixture/no-weights',revision='a'*40,
                                   license='Apache-2.0',license_reviewed=True)
            path=root/'config.json';path.write_text(json.dumps(config))
            with patch('qualify_loader.installed_versions',return_value={}):
                with self.assertRaises(DataError):qualify_loader(path,root/'failure',execute=True)
            report=json.loads((root/'failure/loader-receipt.json').read_text())
            self.assertEqual(report['status'],'failed')
            self.assertFalse(report['actual_forward_backward_completed'])

    def test_prepared_manifest_cannot_substitute_unreviewed_messages(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);rows=[{'id':'a','split':'train','messages':['reviewed']},
                                      {'id':'b','split':'validation','messages':['held validation']}]
            records=root/'records.jsonl';records.write_text('\n'.join(json.dumps(row) for row in rows)+'\n')
            for row in rows:(root/(row['split']+'.jsonl')).write_text(json.dumps(row)+'\n')
            verify_prepared_records(records,root)
            rows[0]['messages']=['unreviewed replacement']
            (root/'train.jsonl').write_text(json.dumps(rows[0])+'\n')
            with self.assertRaises(DataError):verify_prepared_records(records,root)

    def test_collection_requires_every_predeclared_slot_and_hashed_attempt_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);slot={'id':'a','family':'repair-a','split':'train'}
            plan={'schema_version':1,'declared_unix':1,'families':{'repair-a':'train'},'attempts':[slot]}
            file=root/'plan.json';file.write_text(json.dumps(plan))
            ledger={'plan':{'path':'plan.json','sha256':sha256_file(file)},'families':plan['families'],'attempts':[]}
            with self.assertRaisesRegex(DataError,'omits'):collection_receipts(ledger,root)
            ledger['attempts']=[dict(slot,status='passed')]
            with self.assertRaises(DataError):collection_receipts(ledger,root)
            receipt=dict(slot,status='passed',plan_sha256=sha256_file(file),started_unix=2)
            result=root/'attempt.json';result.write_text(json.dumps(receipt))
            ledger['attempts'][0]['evidence']={'path':'attempt.json','sha256':sha256_file(result)}
            self.assertEqual(collection_receipts(ledger,root)['a'],receipt)
            receipt['started_unix']=0;result.write_text(json.dumps(receipt))
            ledger['attempts'][0]['evidence']['sha256']=sha256_file(result)
            with self.assertRaisesRegex(DataError,'predates'):collection_receipts(ledger,root)

    def test_passing_attempt_cannot_supply_different_source_or_actor(self):
        row={'actor':{'kind':'fixture'},'source':{'raw_trace_sha256':'a'*64,
             'transcript_sha256':'b'*64,'snapshot_sha256':'c'*64},'checks':[{'name':'behavior','sha256':'d'*64}]}
        receipt={'status':'passed','actor':row['actor'],'source_sha256':dict(row['source']),
                 'checks':row['checks']}
        bind_record_to_attempt(row,receipt)
        receipt['source_sha256']['snapshot_sha256']='e'*64
        with self.assertRaisesRegex(DataError,'differs'):bind_record_to_attempt(row,receipt)


if __name__=='__main__':unittest.main()
