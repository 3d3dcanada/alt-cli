#!/usr/bin/env python3
"""Offline corpus/attempt accounting and exact-loader readiness; no training."""
import argparse
import json
from pathlib import Path
from prepare import (DataError, REVIEW_FLAGS, canonical, load_jsonl, require,
                     sha256_file, strict_loads, validate_record, verified_file)
from preflight import capability

FLOORS = {'train':32,'validation':8}


def collection_receipts(collection, root):
    """Bind every declared slot to one immutable outcome and its original plan."""
    require(root is not None,'Collection needs an evidence root')
    plan_ref=collection.get('plan',{})
    require(isinstance(plan_ref,dict),'Collection plan reference is missing')
    plan_path=verified_file(Path(root),plan_ref.get('path'),plan_ref.get('sha256'))
    plan=strict_loads(plan_path.read_text())
    require(plan.get('schema_version')==1 and isinstance(plan.get('declared_unix'),(int,float)),
            'Collection plan needs its pre-collection declaration time')
    require(plan.get('families')==collection.get('families'),'Collection families differ from frozen plan')
    planned={}
    for slot in plan.get('attempts',[]):
        require(slot.get('id') not in planned,'Duplicate planned attempt ID')
        require(slot.get('split') in ['train','validation'] and
                plan.get('families',{}).get(slot.get('family'))==slot['split'],
                'Planned family/split mismatch')
        require(not slot['family'].startswith(('final-','sealed-')),'Final families are excluded from collection')
        planned[slot['id']]=slot
    require(bool(planned),'No predeclared collection slots')
    outcomes=collection.get('attempts',[])
    require(len(outcomes)==len(planned) and {a.get('id') for a in outcomes}==set(planned),
            'Collection omits or adds predeclared attempt slots')
    receipts={}
    for attempt in outcomes:
        slot=planned[attempt['id']]
        require(all(attempt.get(k)==slot[k] for k in ['id','family','split']), 'Outcome differs from planned slot')
        if attempt['status']=='unperformed':continue
        evidence=attempt.get('evidence',{})
        require(isinstance(evidence,dict),'Performed attempt has no evidence reference')
        path=verified_file(Path(root),evidence.get('path'),evidence.get('sha256'))
        receipt=strict_loads(path.read_text())
        require(receipt.get('plan_sha256')==plan_ref['sha256'],'Attempt belongs to another collection plan')
        require(all(receipt.get(k)==attempt[k] for k in ['id','family','split','status']),
                'Attempt receipt differs from collection ledger')
        require(isinstance(receipt.get('started_unix'),(int,float)) and
                receipt['started_unix']>=plan['declared_unix'],'Attempt predates collection plan')
        receipts[attempt['id']]=receipt
    return receipts


def bind_record_to_attempt(row, receipt):
    require(receipt is not None and receipt.get('status')=='passed','No hashed passing attempt receipt')
    require(receipt.get('actor')==row.get('actor'),'Actor differs from collected attempt')
    identity={key:row['source'][key] for key in ['raw_trace_sha256','transcript_sha256','snapshot_sha256']}
    require(receipt.get('source_sha256')==identity,'Attempt trace/transcript/source differs from reviewed record')
    checks=sorted([{'name':c['name'],'sha256':c['sha256']} for c in row['checks']],key=lambda c:c['name'])
    require(receipt.get('checks')==checks,'Attempt checks differ from reviewed record')


def evaluate(records=None, registry=None, evidence_root=None, ledger=None,
             config=None, loader_receipt=None, hardware=None):
    report={'schema_version':1,'training_ready':False,'training_run_performed':False,
            'family_floor':FLOORS,'approved_families':{'train':0,'validation':0},
            'review_counts':{},'attempt_counts':{},'blockers':[],'inputs':{},
            'scope':'Readiness only. No weights trained, exported or promoted.'}
    def block(message):report['blockers'].append(message)
    files={'records':records,'registry':registry,'ledger':ledger,'config':config,
           'loader_receipt':loader_receipt}
    for name,path in files.items():
        if path is not None and Path(path).is_file():
            report['inputs'][name]={'path':str(Path(path).resolve()),'sha256':sha256_file(path)}
        else:block('Missing '+name)
    rows=load_jsonl(records) if records and Path(records).is_file() else []
    partitions=strict_loads(Path(registry).read_text()).get('families',{}) if registry and Path(registry).is_file() else {}
    collection=strict_loads(Path(ledger).read_text()) if ledger and Path(ledger).is_file() else {}
    attempts={};groups={'train':set(),'validation':set()};signatures=set()
    for attempt in collection.get('attempts',[]):
        require(attempt.get('id') not in attempts,'Duplicate immutable attempt ID in collection ledger')
        require(attempt.get('status') in ['passed','failed','interrupted','unperformed','infrastructure-failed'],'Invalid attempt status')
        attempts[attempt['id']]=attempt
        status=attempt['status'];report['attempt_counts'][status]=report['attempt_counts'].get(status,0)+1
    if not attempts:block('No complete collection attempt manifest, including failures/unperformed slots')
    receipts={}
    if attempts:
        try:receipts=collection_receipts(collection,evidence_root)
        except (DataError,KeyError,TypeError,ValueError,OSError) as error:
            block('Collection evidence: '+str(error))
    for row in rows:
        review=row.get('review',{}).get('status','missing')
        report['review_counts'][review]=report['review_counts'].get(review,0)+1
        try:
            require(evidence_root is not None,'No evidence root')
            validate_record(row,partitions,Path(evidence_root))
            require(not row['task_group'].startswith(('final-','sealed-')),'Final held-out families cannot enter training')
            signature=canonical({'messages':row['messages'],'tools':row['tools']})
            require(signature not in signatures,'Duplicate trajectory')
            signatures.add(signature)
            link=row.get('collection',{})
            require(link.get('ledger_sha256')==report['inputs'].get('ledger',{}).get('sha256'),'Unbound collection ledger')
            attempt=attempts.get(link.get('attempt_id'))
            require(attempt and attempt.get('status')=='passed','Trajectory needs its actual passing attempt')
            require(attempt.get('family')==row['task_group'] and attempt.get('split')==row['split'],'Collection family/split mismatch')
            require(collection.get('families',{}).get(row['task_group'])==row['split'],'Collection partition mismatch or held-out leakage')
            bind_record_to_attempt(row,receipts.get(attempt['id']))
            groups[row['split']].add(row['task_group'])
        except (DataError,KeyError,TypeError,ValueError,OSError) as error:
            block(f"Record {row.get('id','unknown')}: {error}")
    for split,floor in FLOORS.items():
        report['approved_families'][split]=len(groups[split])
        if len(groups[split])<floor:block(f'{split} needs {floor} independently reviewed families')
    if groups['train']&groups['validation']:block('Family leakage across training/validation')
    hardware=hardware if hardware is not None else capability()
    report['hardware']=hardware
    if hardware.get('torch_cuda_available') is not True:block('Usable CUDA training hardware not measured')
    if config and Path(config).is_file() and loader_receipt and Path(loader_receipt).is_file():
        from train_sft import load_config
        try:configuration=load_config(config)
        except DataError as error:
            block('Training configuration is not reviewed: '+str(error))
            configuration={}
        measured=strict_loads(Path(loader_receipt).read_text())
        if not (measured.get('status')=='loader_step_qualified' and
                measured.get('model')==configuration.get('model') and
                measured.get('config_sha256')==sha256_file(config) and
                measured.get('actual_forward_backward_completed') is True and
                measured.get('weights_format')=='safetensors-parent' and
                measured.get('trust_remote_code') is False):
            block('Exact uncensored Safetensors parent/loader/forward-backward has not been qualified')
        raw=measured.get('raw_evidence')
        if not isinstance(raw,dict) or not Path(raw.get('path','')).is_file() or sha256_file(raw['path'])!=raw.get('sha256'):
            block('Loader measurement evidence missing or changed')
        else:
            observation=strict_loads(Path(raw['path']).read_text())
            devices=hardware.get('cuda_devices',[])
            if len(devices)!=1 or devices[0].get('name')!=observation.get('gpu') or devices[0].get('capability')!=observation.get('compute_capability'):
                block('Current selected GPU differs from the qualified loader device')
    report['training_ready']=not report['blockers']
    return report


def require_training_readiness(receipt, config, dataset):
    require(receipt is not None,'Training requires a fresh --readiness-receipt; see training/readiness.py')
    report=strict_loads(Path(receipt).read_text())
    require(report.get('training_ready') is True,'Training prerequisites remain incomplete')
    require(set(report.get('inputs',{}))=={'records','registry','ledger','config','loader_receipt'},
            'Readiness receipt omits a required bound input')
    for name,entry in report.get('inputs',{}).items():
        require(sha256_file(entry['path'])==entry['sha256'],f'Readiness input changed: {name}')
    require(report.get('inputs',{}).get('config',{}).get('sha256')==sha256_file(config),'Readiness config mismatch')
    prepared=strict_loads((Path(dataset)/'manifest.json').read_text())
    require(prepared.get('records_sha256')==report['inputs']['records']['sha256'],'Prepared corpus differs from reviewed readiness')
    require(prepared.get('split_registry_sha256')==report['inputs']['registry']['sha256'],'Prepared family registry differs from readiness')
    verify_prepared_records(report['inputs']['records']['path'],dataset)
    require(all(report.get('approved_families',{}).get(s,0)>=n for s,n in FLOORS.items()),'Family floors cannot be lowered')
    # Re-evaluate provenance/review evidence and current hardware, not merely a prior boolean.
    inputs={name:Path(value['path']) for name,value in report['inputs'].items()}
    fresh=evaluate(**inputs,evidence_root=report.get('evidence_root'))
    require(fresh['training_ready'],'Readiness no longer holds: '+'; '.join(fresh['blockers']))
    return report


def verify_prepared_records(records, dataset):
    """A rewritten prepared manifest cannot substitute new, unreviewed messages."""
    rows=load_jsonl(records)
    for split in ['train','validation']:
        expected=sorted((row for row in rows if row['split']==split),key=lambda row:row['id'])
        actual=load_jsonl(Path(dataset)/(split+'.jsonl'))
        require(actual==expected,'Prepared '+split+' records differ from reviewed source records')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['records','registry','evidence-root','ledger','config','loader-receipt']:
        parser.add_argument('--'+name,type=Path)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    report=evaluate(args.records,args.registry,args.evidence_root,args.ledger,args.config,args.loader_receipt)
    report['evidence_root']=str(args.evidence_root.resolve()) if args.evidence_root else None
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('x') as stream:json.dump(report,stream,indent=2);stream.write('\n')
    print(json.dumps(report,indent=2))
    return 0 if report['training_ready'] else 2


if __name__=='__main__':raise SystemExit(main())
