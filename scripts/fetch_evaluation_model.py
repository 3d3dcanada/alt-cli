#!/usr/bin/env python3
"""Acquire exactly the selected public uncensored evaluation artifact."""
import argparse, hashlib, urllib.request
from pathlib import Path
from evaluation_models import MODELS
p=argparse.ArgumentParser(description=__doc__); p.add_argument('--model',choices=MODELS,required=True); p.add_argument('--directory',type=Path,required=True); a=p.parse_args()
model=MODELS[a.model]; destination=a.directory/model['filename']; temporary=destination.with_suffix('.partial')
a.directory.mkdir(parents=True,exist_ok=True)
url=f"https://huggingface.co/{model['repository']}/resolve/{model['revision']}/{model['filename']}"
h=hashlib.sha256(); size=0
with urllib.request.urlopen(url,timeout=120) as response,temporary.open('wb') as out:
    while chunk:=response.read(8*1024*1024):
        size+=len(chunk); assert size<=model['bytes'], 'Artifact exceeds pinned size'
        out.write(chunk); h.update(chunk)
assert size==model['bytes'] and h.hexdigest()==model['sha256'], 'Artifact identity mismatch; no fallback'
temporary.replace(destination)
print(f"Verified {model['filename']}: {model['sha256']}")
