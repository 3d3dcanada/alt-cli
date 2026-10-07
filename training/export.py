#!/usr/bin/env python3
"""Opt-in exact-parent adapter merge and pinned llama.cpp F16/Q4_K_M export.

Default validates an export plan without importing Torch or loading weights.
Exported weights remain unqualified until native tools and independent behavior
pass before/after quantization; never replace the selected deployed checkpoint.
"""
import argparse, json, os, signal, subprocess, sys, time
from pathlib import Path
from prepare import require,strict_loads,sha256_file,canonical,DataError

def plan(config_path,run,converter,quantizer,output):
    config=strict_loads(config_path.read_text());receipt=strict_loads((run/'run-receipt.json').read_text());model=config['model']
    require(model.get('uncensored') is True and model.get('license_reviewed') is True,'Review exact uncensored parent and output rights')
    require(receipt.get('status')=='training_finished_unqualified','A completed adapter is required before export')
    require(receipt.get('training_run_performed') is True,'No actual training run exists')
    require(receipt.get('config',{}).get('model')==model,'Adapter ancestry differs from the exact parent')
    adapter=run/'adapter';files=receipt.get('adapter_files',{})
    require(bool(files) and 'adapter_model.safetensors' in files,'Safetensors adapter manifest missing')
    for name,digest in files.items():
        require(Path(name).name==name and sha256_file(adapter/name)==digest,'Adapter files changed')
    require({p.name:sha256_file(p) for p in adapter.iterdir() if p.is_file()}==files,'Adapter contains unrecorded files')
    adapter_config=strict_loads((adapter/'adapter_config.json').read_text())
    require(adapter_config.get('base_model_name_or_path')==model['repository'],'LoRA base does not match parent')
    require(converter.name=='convert_hf_to_gguf.py' and converter.is_file(),'Select an actual llama.cpp conversion script')
    require(quantizer.is_file(),'Select an actual llama.cpp quantizer executable')
    commit=subprocess.check_output(['git','-C',str(converter.parent),'rev-parse','HEAD'],text=True).strip()
    require(len(commit)==40,'Pin the converter source revision')
    merged=output/'merged';f16=output/'model-f16.gguf';q4=output/'model-Q4_K_M.gguf'
    return {'schema_version':1,'status':'planned','model':model,'adapter_manifest':files,'training_receipt_sha256':sha256_file(run/'run-receipt.json'),'config_sha256':sha256_file(config_path),'converter_commit':commit,'converter_sha256':sha256_file(converter),'quantizer_sha256':sha256_file(quantizer),'commands':[[sys.executable,str(converter.resolve()),str(merged.resolve()),'--outfile',str(f16.resolve()),'--outtype','f16'],[str(quantizer.resolve()),str(f16.resolve()),str(q4.resolve()),'Q4_K_M']],'outputs':{'merged':str(merged),'f16':str(f16),'q4':str(q4)},'qualified':False,'scope':'Merge/export job only. No deployment, quality gain or physical GTX1070 claim.'}

def execute(plan,run,output,device,model_class):
    require(not output.exists(),'Keep prior evidence; choose a new export output directory');output.mkdir(parents=True)
    receipt=output/'export-receipt.json'
    def save():receipt.write_text(json.dumps(plan,indent=2)+'\n')
    plan['status']='starting';plan['device']=device;plan['model_class']=model_class;save();started=time.monotonic()
    try:
        import torch,transformers,peft
        from peft import PeftModel
        require(model_class in ('AutoModelForCausalLM','AutoModelForImageTextToText'),'Select a supported exact-parent loader, never remap weights')
        loader=getattr(transformers,model_class,None);require(loader is not None,'Installed Transformers lacks the selected architecture loader')
        model=plan['model'];require(device=='cpu' or torch.cuda.is_available(),'Requested CUDA unavailable')
        plan['libraries']={'torch':torch.__version__,'transformers':transformers.__version__,'peft':peft.__version__};plan['status']='merging';save()
        # Reload the original high-precision parent; never merge into its NF4 training copy.
        parent=loader.from_pretrained(model['repository'],revision=model['revision'],torch_dtype=torch.float16,device_map=device,trust_remote_code=False,use_safetensors=True,low_cpu_mem_usage=True)
        adapter=PeftModel.from_pretrained(parent,str(run/'adapter'),is_trainable=False)
        merged=adapter.merge_and_unload(safe_merge=True);merged.save_pretrained(plan['outputs']['merged'],safe_serialization=True)
        tokenizer=transformers.AutoTokenizer.from_pretrained(model['repository'],revision=model['revision'],trust_remote_code=False);tokenizer.save_pretrained(plan['outputs']['merged'])
        if model_class=='AutoModelForImageTextToText':
            processor=transformers.AutoProcessor.from_pretrained(model['repository'],revision=model['revision'],trust_remote_code=False);processor.save_pretrained(plan['outputs']['merged'])
        plan['status']='converting';save()
        with (output/'conversion.log').open('w') as log:
            for command in plan['commands']:
                pinned=plan['converter_sha256'] if command[0]==sys.executable else plan['quantizer_sha256'];file=Path(command[1] if command[0]==sys.executable else command[0]);require(sha256_file(file)==pinned,'Pinned conversion executable changed')
                process=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
                try:
                    code=process.wait(timeout=3600);require(code==0,'Conversion command failed; inspect conversion.log')
                finally:
                    try:os.killpg(process.pid,signal.SIGKILL)
                    except ProcessLookupError:pass
                    process.wait()
        plan['output_hashes']={name:sha256_file(Path(path)) for name,path in plan['outputs'].items() if name!='merged'}
        plan['merged_manifest']={str(p.relative_to(plan['outputs']['merged'])):sha256_file(p) for p in Path(plan['outputs']['merged']).rglob('*') if p.is_file()}
        plan['status']='exported_unqualified'
    except BaseException as e:
        plan['status']='interrupted' if isinstance(e,KeyboardInterrupt) else 'failed';plan['error_type']=type(e).__name__;save();raise
    finally:plan['elapsed_seconds']=time.monotonic()-started;save()
    return plan

def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['config','run','converter','quantizer','output']:p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--execute',action='store_true');p.add_argument('--device',choices=['cpu','cuda:0'],default='cpu');p.add_argument('--model-class',choices=['AutoModelForCausalLM','AutoModelForImageTextToText'],default='AutoModelForCausalLM');a=p.parse_args()
    job=plan(a.config,a.run,a.converter,a.quantizer,a.output)
    if a.execute:job=execute(job,a.run,a.output,a.device,a.model_class)
    print(json.dumps(job,indent=2))

if __name__=='__main__':main()
