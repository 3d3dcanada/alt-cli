#!/usr/bin/env python3
"""Explicit one-step exact-parent GPU loader qualification; no saved adapter.

Default records a plan only. --execute loads the exact reviewed Safetensors
parent, creates a disposable adapter and performs one finite backward pass.
This is stack evidence, not coding quality or a useful trained checkpoint.
"""
import argparse
import json
from pathlib import Path
import time
from prepare import require, sha256_file
from train_sft import load_config, installed_versions, REFERENCE_PACKAGES


def run(config_path, output, execute=False, cache_dir=None):
    config=load_config(config_path)
    require(not output.exists(),'Choose a new loader evidence directory')
    output.mkdir(parents=True)
    report={'schema_version':1,'status':'planned','model':config['model'],
            'config_sha256':sha256_file(config_path),'weights_format':'safetensors-parent',
            'trust_remote_code':False,'actual_forward_backward_completed':False,
            'training_run_performed':False,'adapter_saved':False,'quality_qualified':False,
            'scope':'Synthetic one-step stack qualification; no model-quality measurement'}
    receipt=output/'loader-receipt.json';raw=output/'step-observation.json'
    def save():receipt.write_text(json.dumps(report,indent=2)+'\n')
    save()
    if not execute:return report
    started=time.monotonic()
    try:
        versions=installed_versions();require(versions==REFERENCE_PACKAGES,'Qualify exact package pins first')
        import torch
        from transformers import AutoConfig, AutoTokenizer, AutoModelForCausalLM, BitsAndBytesConfig
        from peft import LoraConfig, get_peft_model, prepare_model_for_kbit_training
        require(torch.cuda.is_available() and torch.cuda.device_count()==1,'Select one usable CUDA device')
        model=config['model'];options={'revision':model['revision'],'trust_remote_code':False,
                                      'cache_dir':str(cache_dir) if cache_dir else None}
        architecture=AutoConfig.from_pretrained(model['repository'],**options)
        require(architecture.architectures and all(n.endswith('ForCausalLM') for n in architecture.architectures),
                'Exact architecture needs a separately qualified loader; no substitution')
        dtype=torch.float16 if config['training']['compute_dtype']=='fp16' else torch.bfloat16
        require(dtype!=torch.bfloat16 or torch.cuda.is_bf16_supported(),'Selected GPU lacks BF16')
        tokenizer=AutoTokenizer.from_pretrained(model['repository'],**options)
        quantization=BitsAndBytesConfig(load_in_4bit=True,bnb_4bit_quant_type='nf4',
                                      bnb_4bit_use_double_quant=True,bnb_4bit_compute_dtype=dtype)
        parent=AutoModelForCausalLM.from_pretrained(model['repository'],**options,
                    use_safetensors=True,device_map={'':0},torch_dtype=dtype,
                    quantization_config=quantization,attn_implementation='eager')
        parent=prepare_model_for_kbit_training(parent)
        adapted=get_peft_model(parent,LoraConfig(r=4,lora_alpha=8,target_modules='all-linear',task_type='CAUSAL_LM'))
        tokens=tokenizer('Loader qualification only. One plus one equals two.',return_tensors='pt').to('cuda:0')
        tokens['labels']=tokens['input_ids'].clone()
        torch.cuda.reset_peak_memory_stats();adapted.train();loss=adapted(**tokens).loss
        require(bool(torch.isfinite(loss).item()),'Non-finite qualification loss')
        loss.backward()
        gradients=[p.grad for p in adapted.parameters() if p.requires_grad and p.grad is not None]
        require(bool(gradients) and all(bool(torch.isfinite(g).all().item()) for g in gradients),'Missing/non-finite adapter gradients')
        observation={'packages':versions,'architectures':architecture.architectures,
                     'loss':float(loss.detach().cpu()),'trainable_gradients':len(gradients),
                     'gpu':torch.cuda.get_device_name(0),'compute_capability':list(torch.cuda.get_device_capability(0)),
                     'peak_allocated_bytes':torch.cuda.max_memory_allocated(),'tokens':tokens['input_ids'].numel()}
        raw.write_text(json.dumps(observation,indent=2)+'\n')
        report.update(status='loader_step_qualified',actual_forward_backward_completed=True,
                      raw_evidence={'path':str(raw.resolve()),'sha256':sha256_file(raw)})
    except BaseException as error:
        report.update(status='interrupted' if isinstance(error,KeyboardInterrupt) else 'failed',error_type=type(error).__name__)
        raise
    finally:report['elapsed_seconds']=time.monotonic()-started;save()
    return report


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--execute',action='store_true');parser.add_argument('--cache-dir',type=Path)
    args=parser.parse_args();print(json.dumps(run(args.config,args.output,args.execute,args.cache_dir),indent=2))


if __name__=='__main__':main()
