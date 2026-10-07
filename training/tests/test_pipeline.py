import copy,hashlib,json,sys,tempfile,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from capture import response,normalize
from rewards import score
from prepare import DataError

class PipelineTests(unittest.TestCase):
    def test_native_fragments_and_reasoning_are_preserved(self):
        frames=[{'choices':[{'index':0,'delta':{'reasoning_content':'Inspect.','tool_calls':[{'index':0,'id':'c','function':{'name':'echo','arguments':'{"text":'}}]},'finish_reason':None}]},{'choices':[{'index':0,'delta':{'tool_calls':[{'index':0,'function':{'arguments':'"☃"}'}}]},'finish_reason':'tool_calls'}]}]
        raw=''.join('data: '+json.dumps(f)+'\n\n' for f in frames)+'data: [DONE]\n\n'
        result=normalize([response(raw,True)])[0]
        self.assertEqual(result['reasoning_content'],'Inspect.')
        self.assertEqual(result['tool_calls'][0]['function']['arguments'],{'text':'☃'})
        with self.assertRaises(DataError):response(raw.replace('data: [DONE]\n\n',''),True)
    def fixture(self,root):
        def put(name,row):
            p=root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(row));return {'path':name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
        put('oracle/check.py',{'assertion':'behavior'});oracle={'check.py':hashlib.sha256((root/'oracle/check.py').read_bytes()).hexdigest()}
        raw=put('raw.json',{'actual':'passed'});source=put('source.json',{'solution':'fixed'})
        check={'schema_version':1,'kind':'independent_behavior','passed':True,'exit_code':0,'tests_run':1,'source_sha256':source['sha256'],'raw_output_path':raw['path'],'raw_output_sha256':raw['sha256'],'oracle_inputs':oracle}
        record={'schema_version':1,'id':'fixture','rights_reviewed':True,'gold':put('gold.json',check),'candidate':put('candidate.json',check),'source':source,'baseline_sha256':'a'*64,'oracle_before':oracle,'oracle_after':oracle,'oracle_directory':'oracle','completion_claim_supported':True,'cost':{'generated_tokens':20,'generated_token_budget':100}}
        return record,check,put
    def test_only_correct_current_unchanged_behavior_receives_efficiency_bonus(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);record,check,put=self.fixture(root);result=score(record,root)
            self.assertTrue(result['eligible']);self.assertAlmostEqual(result['reward'],1.08)
            check['passed']=False;check['exit_code']=1;record['candidate']=put('candidate.json',check)
            result=score(record,root);self.assertTrue(result['eligible']);self.assertEqual(result['reward'],0)
    def test_invalid_gold_is_excluded_never_rewarded(self):
        for mutation in [{'passed':False},{'exit_code':True},{'tests_run':0},{'timed_out':True},{'complete':False},{'cancelled':True}]:
            with self.subTest(mutation=mutation),tempfile.TemporaryDirectory() as d:
                root=Path(d);record,check,put=self.fixture(root);check.update(mutation);record['gold']=put('gold.json',check);result=score(record,root)
                self.assertFalse(result['eligible']);self.assertIsNone(result['reward'])
    def test_stale_changed_tests_noop_claim_and_missing_rights_are_excluded(self):
        for mutation in ['stale','tests','noop','claim','rights','raw']:
            with self.subTest(mutation=mutation),tempfile.TemporaryDirectory() as d:
                root=Path(d);record,check,put=self.fixture(root)
                if mutation=='stale':check['source_sha256']='b'*64;record['candidate']=put('candidate.json',check)
                if mutation=='tests':record['oracle_after']={'check.py':'b'*64}
                if mutation=='noop':record['baseline_sha256']=record['source']['sha256']
                if mutation=='claim':record['completion_claim_supported']=False
                if mutation=='rights':record['rights_reviewed']=False
                if mutation=='raw':(root/'raw.json').write_text('changed')
                result=score(record,root);self.assertFalse(result['eligible']);self.assertIsNone(result['reward'])

if __name__=='__main__':unittest.main()
