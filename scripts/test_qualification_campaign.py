#!/usr/bin/env python3
"""Deterministic campaign-integrity controls; no weights or quality claims."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
import subprocess
import time
from unittest.mock import patch
import qualification_campaign as campaign


class CampaignTests(unittest.TestCase):
    def claim(self, root, slot):
        campaign.write_new(root/'attempts'/slot['id']/'started.json', {'slot':slot,'campaign_sha256':campaign.sha(root/'campaign.json'),'attempt_number':1,'started_unix':time.time()})

    def make_campaign(self, root):
        inputs=root/'inputs';inputs.mkdir()
        for name in ['baseline','candidate','engine','runtime','one.gguf','two.gguf']:
            (inputs/name).write_bytes(('protocol-fixture-no-weights:'+name).encode())
        models={'one':('one.gguf',campaign.sha(inputs/'one.gguf')),
                'two':('two.gguf',campaign.sha(inputs/'two.gguf'))}
        with patch.object(campaign,'MODELS',models):
            manifest=campaign.create(root/'campaign',inputs/'baseline',inputs/'engine',
                                     inputs/'runtime',inputs)
        return root/'campaign',inputs,manifest

    def test_thirty_independent_final_families_and_complete_slot_grid(self):
        tasks=campaign.task_manifest()
        self.assertEqual(sum(t['partition']=='final-holdout' for t in tasks),30)
        self.assertEqual(len({t['family'] for t in tasks}),len(tasks))
        slots=campaign.attempt_slots(tasks,3)
        self.assertEqual(sum(s['partition']=='final-holdout' for s in slots),360)
        self.assertEqual(len({s['id'] for s in slots}),len(slots))
        self.assertTrue({'middle-turn-correction','dirty-workspace','navigation','multi-file','configuration','numeric-boundary'}<={t['category'] for t in tasks})

    def test_final_refuses_to_run_before_candidate_seal_without_claiming_a_slot(self):
        with tempfile.TemporaryDirectory() as directory:
            root,_,_=self.make_campaign(Path(directory))
            with self.assertRaises(FileNotFoundError):
                campaign.run_slots(root,'final-holdout',1,['baseline'])
            self.assertFalse((root/'attempts').exists())

    def test_candidate_is_immutable_and_cannot_be_sealed_after_final_inspection(self):
        with tempfile.TemporaryDirectory() as directory:
            root,inputs,manifest=self.make_campaign(Path(directory))
            slot=next(s for s in manifest['attempts'] if s['partition']=='final-holdout')
            self.claim(root,slot)
            with self.assertRaises(campaign.InvalidCampaign):
                campaign.seal(root,inputs/'candidate','a'*40,False)
            self.assertFalse((root/'candidate-seal.json').exists())
        with tempfile.TemporaryDirectory() as directory:
            root,inputs,manifest=self.make_campaign(Path(directory))
            campaign.seal(root,inputs/'candidate','a'*40,False)
            (root/'binaries/candidate').write_bytes(b'changed')
            with self.assertRaises(campaign.InvalidCampaign):campaign.checked_seal(root,manifest)

    def test_missing_and_interrupted_slots_stay_in_the_denominator(self):
        with tempfile.TemporaryDirectory() as directory:
            root,_,manifest=self.make_campaign(Path(directory))
            slot=next(s for s in manifest['attempts'] if s['partition']=='development' and s['arm']=='baseline')
            self.claim(root,slot)
            result=campaign.summarize(root)
            self.assertEqual(result['status_counts']['interrupted'],1)
            self.assertEqual(result['status_counts']['unperformed'],len(manifest['attempts'])-1)
            self.assertFalse(result['preset_promoted'])
            self.assertTrue(all(v['uncertainty'] is None for v in result['comparisons'].values()))

    def test_hash_changed_or_extra_raw_evidence_invalidates_an_attempt(self):
        with tempfile.TemporaryDirectory() as directory:
            root,_,manifest=self.make_campaign(Path(directory));slot=next(s for s in manifest['attempts'] if s['partition']=='development' and s['arm']=='baseline')
            folder=root/'attempts'/slot['id'];self.claim(root,slot)
            campaign.write_new(folder/'result.json',{'slot':slot,**campaign.observed_outcome(None,slot),
                               'evidence_sha256':campaign.file_manifest(folder)})
            campaign.summarize(root)
            (folder/'started.json').write_text('{}')
            with self.assertRaises(campaign.InvalidCampaign):campaign.summarize(root)

    def test_final_receipts_require_the_prior_candidate_seal(self):
        with tempfile.TemporaryDirectory() as directory:
            root,_,manifest=self.make_campaign(Path(directory))
            slot=next(s for s in manifest['attempts'] if s['partition']=='final-holdout')
            self.claim(root,slot)
            with self.assertRaises(FileNotFoundError):campaign.summarize(root)

    def test_controller_is_frozen_with_the_oracles(self):
        with tempfile.TemporaryDirectory() as directory:
            root,_,_=self.make_campaign(Path(directory))
            controller=root/'evaluator/qualification_campaign.py'
            self.assertEqual(campaign.sha(controller),campaign.sha(campaign.__file__))
            controller.write_text('print("changed")')
            with self.assertRaises(campaign.InvalidCampaign):campaign.verify(root)

    def test_shards_keep_both_arms_of_every_repeat_together(self):
        slots=campaign.attempt_slots(campaign.task_manifest(),3)
        for count in [4,16,32]:
            observed={}
            for index,slot in enumerate(slots):
                key=(slot['family'],slot['model'],slot['repetition'])
                shard=(index//2)%count
                if key in observed:self.assertEqual(observed[key],shard)
                observed[key]=shard

    def test_two_arm_driver_records_actual_outcomes_and_never_replays_claimed_slots(self):
        with tempfile.TemporaryDirectory() as directory:
            root,inputs,manifest=self.make_campaign(Path(directory))
            campaign.seal(root,inputs/'candidate','a'*40,False)
            commands=[]
            def fixture_evaluator(command,out,err,seconds):
                commands.append(command)
                output=Path(command[command.index('--output')+1])/'fixture-attempt'
                output.mkdir(parents=True)
                campaign.write_new(output/'report.json',{'case':command[command.index('--cases')+1],'oracle_unchanged':True,
                    'before':{'passed':False,'exit':1},'after':{'passed':False,'exit':1},'passed':False,
                    'scores':{'turn_completed':True,'stop_reason':'end_turn'},
                    'scope':'Deterministic orchestration fixture; no weights'})
                return subprocess.CompletedProcess(command,1)
            with patch.object(campaign,'run_evaluator',side_effect=fixture_evaluator):
                self.assertEqual(campaign.run_slots(root,'development',2),2)
            self.assertEqual(len(commands),2)
            self.assertEqual({Path(cmd[cmd.index('--binary')+1]).name for cmd in commands},{'baseline','candidate'})
            for command in commands:
                self.assertEqual(command[command.index('--partition')+1],'development')
                self.assertEqual(command[command.index('--timeout')+1],'600')
            summary=campaign.summarize(root)
            self.assertEqual(summary['status_counts']['measured'],2)
            self.assertEqual(sum(row['behavior_passed'] is True for row in summary['attempts']),0)
            before={str(p):campaign.sha(p) for p in (root/'attempts').glob('*/result.json')}
            with patch.object(campaign,'run_evaluator',side_effect=fixture_evaluator):
                self.assertEqual(campaign.run_slots(root,'development',2),2)
            self.assertEqual(len(commands),4)
            self.assertTrue(all(campaign.sha(p)==digest for p,digest in before.items()))

    def test_editing_only_result_cannot_turn_a_missing_report_into_a_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            root,_,manifest=self.make_campaign(Path(directory))
            slot=next(s for s in manifest['attempts'] if s['partition']=='development' and s['arm']=='baseline')
            self.claim(root,slot);folder=root/'attempts'/slot['id']
            outcome={'slot':slot,**campaign.observed_outcome(None,slot),'evidence_sha256':campaign.file_manifest(folder)}
            campaign.write_new(folder/'result.json',outcome)
            campaign.summarize(root)
            outcome.update(status='measured',behavior_passed=True)
            (folder/'result.json').write_text(json.dumps(outcome))
            with self.assertRaisesRegex(campaign.InvalidCampaign,'independent report'):campaign.summarize(root)

    def test_start_from_a_different_campaign_cannot_enter_accounting(self):
        with tempfile.TemporaryDirectory() as directory:
            root,_,manifest=self.make_campaign(Path(directory))
            slot=next(s for s in manifest['attempts'] if s['partition']=='development' and s['arm']=='baseline')
            self.claim(root,slot);path=root/'attempts'/slot['id']/'started.json'
            value=campaign.load(path);value['campaign_sha256']='a'*64;path.write_text(json.dumps(value))
            with self.assertRaisesRegex(campaign.InvalidCampaign,'another campaign'):campaign.summarize(root)

    def test_final_start_cannot_predate_the_candidate_seal(self):
        with tempfile.TemporaryDirectory() as directory:
            root,inputs,manifest=self.make_campaign(Path(directory))
            sealed=campaign.seal(root,inputs/'candidate','a'*40,False)
            slot=next(s for s in manifest['attempts'] if s['partition']=='final-holdout')
            self.claim(root,slot);path=root/'attempts'/slot['id']/'started.json'
            value=campaign.load(path);value['started_unix']=sealed['sealed_unix']-1;path.write_text(json.dumps(value))
            with self.assertRaisesRegex(campaign.InvalidCampaign,'predates'):campaign.summarize(root)

    def test_family_bootstrap_does_not_treat_three_repeats_as_three_families(self):
        result=campaign.paired_family_interval([1/3]*30,samples=1000)
        self.assertEqual(result['families'],30)
        self.assertAlmostEqual(result['mean_lift'],1/3)
        self.assertAlmostEqual(result['bootstrap_95_percent'][0],1/3)
        self.assertIn('within-family',result['unit'])

    def test_exclusive_receipts_cannot_rewrite_failure_into_success(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'outcome.json';campaign.write_new(path,{'passed':False})
            with self.assertRaises(FileExistsError):campaign.write_new(path,{'passed':True})
            self.assertFalse(json.loads(path.read_text())['passed'])


if __name__=='__main__':unittest.main()
