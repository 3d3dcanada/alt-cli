#!/usr/bin/env python3
"""Actual source/oracle acceptance for deterministic adapter protocol fixtures."""
import json,subprocess,sys,tempfile,unittest
from pathlib import Path
from acceptance_v5 import CASES
from qualify_adapter import sha,resources
from optimize_instructions import proposal_error,completion_error

class OptimizerTests(unittest.TestCase):
    def test_transport_success_cannot_accept_an_unfinished_proposal(self):
        complete=[{'type':'turn_end','data':{'stopReason':'end_turn'}}];receipts=[{'complete':True,'status':200}]
        self.assertIsNone(completion_error(complete,receipts))
        for events,cost in [([],receipts),([{'type':'turn_end','data':{'stopReason':'max_tokens'}}],receipts),(complete,[]),(complete,[{'complete':False,'status':200}]),(complete,[{'complete':True,'status':200,'provider_budget_violation':True}])]:
            with self.subTest(events=events,cost=cost):self.assertIsNotNone(completion_error(events,cost))
    def test_optimization_cannot_disable_execution_evidence_gates(self):
        for script in ('optimize_instructions.py','qualify_adapter.py','collect_live_evidence.py','campaign_v5.py','live_acceptance.py','acceptance_v5.py'):
            result=subprocess.run([sys.executable,'-O',str(Path(__file__).with_name(script)),'--help'],capture_output=True,text=True,timeout=10)
            self.assertNotEqual(result.returncode,0);self.assertIn('evidence assertions must remain enabled',result.stderr)
    def test_tool_markup_is_not_an_instruction_proposal(self):
        self.assertIsNotNone(proposal_error('<function=alt__list></function></tool_call>'))
        self.assertIsNotNone(proposal_error('<tool_calls>invented</tool_calls>'))
        self.assertIsNone(proposal_error('Read the current range before editing. Use actual check evidence to decide the next step.'))
    def test_empty_oversized_and_control_responses_are_rejected(self):
        for text in (' ', '☃'*1667, 'Read\x00source'):
            with self.subTest(text=text[:20]):self.assertIsNotNone(proposal_error(text))

class AdapterTests(unittest.TestCase):
    def execute(self,mutation):
        with tempfile.TemporaryDirectory(prefix='alt-adapter-test-') as d:
            root=Path(d);adapter=root/'adapter.py'
            prefix="import json,os\nfrom pathlib import Path\nrequest=json.loads(Path(os.environ['ALT_ADAPTER_INPUT']).read_text())\n"
            body=''
            if mutation!='prose':body+='\n'.join(f'Path({name!r}).write_text({text!r})' for name,text in CASES['python-feature']['fixed'].items())+'\n'
            if mutation=='oracle':body+="Path('../independent/check.py').write_text('print(\\\"fake pass\\\")')\n"
            output="Path(os.environ['ALT_ADAPTER_OUTPUT']).write_text('broken JSON')" if mutation=='malformed' else "Path(os.environ['ALT_ADAPTER_OUTPUT']).write_text(json.dumps({'all_model_calls_captured':True,'inference':[],'score':1.0,'claim':'everything passed'}))"
            adapter.write_text(prefix+body+output+'\n')
            manifest={'schema':1,'kind':'deterministic-solver','command':[sys.executable,str(adapter)],'executable_sha256':sha(Path(sys.executable).resolve()),'source_files':{str(adapter):sha(adapter)},'source_reviewed':True,'license_reviewed':True,'roles':[]}
            file=root/'manifest.json';file.write_text(json.dumps(manifest));destination=root/'run'
            process=subprocess.run([sys.executable,str(Path(__file__).with_name('qualify_adapter.py')),'--manifest',str(file),'--output',str(destination),'--cases','python-feature','--timeout','10','--allow-host'],capture_output=True,text=True,timeout=20)
            self.assertTrue((destination/'summary.json').exists(),process.stderr)
            return process.returncode,json.loads((destination/'summary.json').read_text())['rows'][0]
    def test_fluent_scores_without_source_changes_cannot_pass(self):
        code,row=self.execute('prose');self.assertEqual(code,1);self.assertFalse(row['passed']);self.assertFalse(row['qualified'])
    def test_actual_source_execution_and_unchanged_oracle_are_required(self):
        code,row=self.execute('correct');self.assertEqual(code,0);self.assertTrue(row['qualified']);self.assertTrue(row['after']['completion_observed'])
        code,row=self.execute('oracle');self.assertEqual(code,1);self.assertFalse(row['oracle_unchanged']);self.assertFalse(row['qualified'])
    def test_malformed_resource_output_is_retained_and_not_qualified(self):
        code,row=self.execute('malformed');self.assertEqual(code,1);self.assertTrue(row['passed']);self.assertFalse(row['qualified']);self.assertTrue(row['resources']['errors'])
    def test_undeclared_missing_or_malformed_calls_are_not_resource_visibility(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d)
            for output,roles in [({'all_model_calls_captured':True,'inference':[]},[{'id':'student'}]),({'all_model_calls_captured':True,'inference':[{}]},[]),({'all_model_calls_captured':True,'inference':'fake'},[])]:
                with self.subTest(output=output):self.assertFalse(resources(output,roles,root)['complete_visibility'])

if __name__=='__main__':unittest.main()
