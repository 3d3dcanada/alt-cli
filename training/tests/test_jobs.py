"""Offline boundary integration fixtures; no trained weights or model quality claim."""
import copy,json,subprocess,sys,tempfile,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from prepare import DataError,canonical,sha256_file,validate_arguments
from capture import capture
from train_sft import validate_resume
from export import plan

class JobTests(unittest.TestCase):
    def test_schema_rejects_bool_as_integer_extra_fields_and_bad_native_actions(self):
        schema={'type':'object','properties':{'action':{'type':'string','enum':['replace','read']},'line':{'type':'integer','minimum':1},'names':{'type':'array','items':{'type':'string'},'uniqueItems':True}},'required':['action','line'],'additionalProperties':False}
        validate_arguments({'action':'replace','line':1,'names':['é','☃']},schema)
        for arguments in [{'action':'read','line':True},{'action':'destroy','line':1},{'action':'read','line':0},{'action':'read','line':1,'unknown':1},{'action':'read','line':1,'names':['a','a']}]:
            with self.subTest(arguments=arguments),self.assertRaises(DataError):validate_arguments(arguments,schema)
        with self.assertRaises(DataError):validate_arguments({}, {'$ref':'https://example.invalid/schema'})
    def test_checkpoint_resume_uses_exact_dataset_config_and_complete_files(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);checkpoint=root/'checkpoint-10';checkpoint.mkdir();(checkpoint/'optimizer.pt').write_bytes(b'fixture-not-torch');(checkpoint/'trainer_state.json').write_text('{"step":10}')
            config={'model':{'revision':'a'*40}};receipt=root/'run-receipt.json'
            data={'config':config,'dataset_manifest_sha256':'b'*64,'checkpoint_manifests':{str(checkpoint.resolve()):{p.name:sha256_file(p) for p in checkpoint.iterdir()}}};receipt.write_text(canonical(data))
            validate_resume(checkpoint,receipt,config,'b'*64)
            with self.assertRaises(DataError):validate_resume(checkpoint,receipt,config,'c'*64)
            with self.assertRaises(DataError):validate_resume(checkpoint,receipt,{'model':{'revision':'d'*40}},'b'*64)
            (checkpoint/'optimizer.pt').write_bytes(b'corrupted')
            with self.assertRaises(DataError):validate_resume(checkpoint,receipt,config,'b'*64)
    def test_export_refuses_missing_training_wrong_ancestry_and_changed_adapter(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);run=root/'run';adapter=run/'adapter';adapter.mkdir(parents=True);converter_dir=root/'llama';converter_dir.mkdir();converter=converter_dir/'convert_hf_to_gguf.py';converter.write_text('# fixture only\n');quantizer=root/'llama-quantize';quantizer.write_bytes(b'fixture-not-executable')
            subprocess.run(['git','init','-q',str(converter_dir)],check=True,capture_output=True);subprocess.run(['git','-C',str(converter_dir),'add',converter.name],check=True,capture_output=True);subprocess.run(['git','-C',str(converter_dir),'-c','user.name=Fixture','-c','user.email=fixture@invalid','commit','-qm','fixture'],check=True,capture_output=True)
            model={'repository':'fixture/model','revision':'a'*40,'uncensored':True,'license_reviewed':True};config=root/'config.json';config.write_text(canonical({'model':model}))
            (adapter/'adapter_model.safetensors').write_bytes(b'fixture-not-weights');(adapter/'adapter_config.json').write_text(canonical({'base_model_name_or_path':model['repository']}))
            receipt={'status':'training_finished_unqualified','training_run_performed':False,'config':{'model':model},'adapter_files':{p.name:sha256_file(p) for p in adapter.iterdir()}};file=run/'run-receipt.json';file.write_text(canonical(receipt))
            with self.assertRaises(DataError):plan(config,run,converter,quantizer,root/'export')
            receipt['training_run_performed']=True;file.write_text(canonical(receipt));result=plan(config,run,converter,quantizer,root/'export');self.assertFalse(result['qualified']);self.assertFalse((root/'export').exists())
            receipt['config']['model']=dict(model,revision='b'*40);file.write_text(canonical(receipt))
            with self.assertRaises(DataError):plan(config,run,converter,quantizer,root/'export')
            receipt['config']['model']=model;file.write_text(canonical(receipt));(adapter/'adapter_model.safetensors').write_bytes(b'changed')
            with self.assertRaises(DataError):plan(config,run,converter,quantizer,root/'export')
    def capture_fixture(self,root):
        attempt=root/'campaign'/'python-feature';(attempt/'project').mkdir(parents=True);(attempt/'independent').mkdir();(attempt/'state'/'inference'/'session').mkdir(parents=True)
        def put(path,value):
            destination=attempt/path;destination.write_text(canonical(value));return sha256_file(destination)
        (attempt.parent/'configuration.json').write_text(canonical({'partition':'development','sha256':'a'*64}));(attempt/'project'/'solution.py').write_text('fixed\n');(attempt/'independent'/'check.py').write_text('# actual capture boundary fixture\n')
        oracle={'check.py':sha256_file(attempt/'independent'/'check.py')}
        put('fixture.json',{'oracle_version':5,'oracle_sha256_before':oracle})
        put('report.json',{'case':'python-feature','task_group':'python-feature','passed':True,'oracle_unchanged':True,'after':{'completion_observed':True,'exit':0,'stdout':'fixture behavioral oracle completed'},'resulting_source':{'solution.py':'fixed\n'}})
        put('turn.jsonl',{'fixture':'not a live model result'})
        tool={'type':'function','function':{'name':'read','parameters':{'type':'object','properties':{'path':{'type':'string'}},'required':['path'],'additionalProperties':False}}}
        call={'role':'assistant','content':None,'tool_calls':[{'id':'c','type':'function','function':{'name':'read','arguments':'{"path":"solution.py"}'}}]}
        request={'model':'fixture-no-weights','messages':[{'role':'user','content':'Inspect solution.py'},call,{'role':'tool','tool_call_id':'c','content':'fixed\n'}],'tools':[tool],'stream':False}
        response={'choices':[{'message':{'role':'assistant','content':'Recorded source; independent fixture check completed.'},'finish_reason':'stop'}]}
        prefix='state/inference/session/call';req=put(prefix+'-request.json',request);raw=put(prefix+'-response.raw',response);put(prefix+'-receipt.json',{'complete':True,'status':200,'request_sha256':req,'response_sha256':raw,'response_truncated':False})
        manifest={str(p.relative_to(attempt)):sha256_file(p) for p in attempt.rglob('*') if p.is_file() and 'project' not in p.relative_to(attempt).parts};put('evidence-sha256.json',manifest)
        return attempt,{'kind':'model','repository':'fixture-no-weights','uncensored':True,'artifact_sha256':'a'*64}, {'repository':'fixture://boundary-test','revision':'b'*40,'license':'Apache-2.0'}
    def test_capture_preserves_native_pairing_and_keeps_every_review_pending(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);attempt,actor,source=self.capture_fixture(root);record=capture(attempt,actor,source,'python-feature','train',root/'pending')
            self.assertEqual(record['review']['status'],'pending');self.assertFalse(record['review']['tool_results_real']);self.assertEqual(record['messages'][1]['tool_calls'][0]['function']['arguments'],{'path':'solution.py'});self.assertEqual(record['messages'][2]['tool_call_id'],'c')
    def test_capture_rejects_stale_source_altered_oracle_and_sealed_renaming(self):
        for mutation in ['source','oracle','sealed']:
            with self.subTest(mutation=mutation),tempfile.TemporaryDirectory() as d:
                root=Path(d);attempt,actor,source=self.capture_fixture(root)
                if mutation=='source':(attempt/'project/solution.py').write_text('stale\n')
                if mutation=='oracle':(attempt/'independent/check.py').write_text('changed')
                family='sealed-intervals' if mutation=='sealed' else 'python-feature'
                with self.assertRaises(DataError):capture(attempt,actor,source,family,'train',root/'pending')

if __name__=='__main__':unittest.main()
