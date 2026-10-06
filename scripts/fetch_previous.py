#!/usr/bin/env python3
import argparse,hashlib,urllib.request
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--url',required=True);p.add_argument('--sha256',required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
assert a.url.startswith('https://') and len(a.sha256)==64 and all(c in '0123456789abcdefABCDEF' for c in a.sha256)
h=hashlib.sha256();size=0
with urllib.request.urlopen(a.url,timeout=60) as response,a.output.open('xb') as out:
 assert response.url.startswith('https://'),'TLS downgrade'
 while chunk:=response.read(1024*1024):
  size+=len(chunk);assert size<=128*1024*1024,'Package too large';h.update(chunk);out.write(chunk)
assert h.hexdigest()==a.sha256.lower(),'Previous package integrity mismatch'
