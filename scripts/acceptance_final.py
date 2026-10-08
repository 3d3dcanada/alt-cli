#!/usr/bin/env python3
"""Final-pass family contracts. Do not inspect model outcomes until candidate seal.

Reference implementations qualify the oracle only; they are never model prompts.
Historical development/validation families are disjoint from the 30 new contracts.
"""
import copy
import json
import subprocess
import tempfile
import uuid
from pathlib import Path
import acceptance_v5 as previous
from acceptance_projects import check, oracle_inputs, assertion_script, write

ORACLE_VERSION = 'final-pass-2'
CASES = {name: copy.deepcopy(previous.CASES[name]) for name in
         ['python-multifile', 'python-dependency', 'rust-feature', 'javascript-config']}
for name, case in CASES.items():
    case['partition'] = 'development' if name.startswith('python-') else 'validation'
    case['task_group'] = name


def add(family, signature, goal, fixed, oracle, *, files=None, target='solution.py',
        category='feature', followup=None, dirty=False, preserve=()):
    name = 'final-' + family
    body = dict(files or {})
    body[target] = f'def {signature}:\n    raise NotImplementedError({family!r})\n'
    for path in preserve:
        if path not in body or path == target:
            raise ValueError('Preserved fixture file must be a seeded dependency')
        oracle += ('\nfrom pathlib import Path\n'
                   f'assert Path({path!r}).read_bytes() == {body[path].encode()!r}, '
                   f'{("Protected fixture changed: " + path)!r}\n')
    CASES[name] = {'goal': goal, 'files': body, 'fixed': {target: fixed},
                   'oracle': oracle, 'partition': 'final-holdout', 'task_group': name,
                   'category': category, 'followup': followup, 'dirty': dirty,
                   'preserved_files': list(preserve),
                   'interface': 'tui' if followup else 'cli'}
    if followup:
        CASES[name]['prompt_suffix'] = ' First inspect the files and acknowledge the requested API only. Wait for my next message before implementing or running acceptance; I will clarify this requirement.'


add('rational-normalization', 'normalize(numerator, denominator)',
    'Implement normalize(numerator, denominator) in solution.py. Return a reduced integer (numerator, denominator) tuple with positive denominator; zero is (0,1). A zero denominator raises ZeroDivisionError.',
    'from math import gcd\ndef normalize(numerator, denominator):\n if denominator==0: raise ZeroDivisionError()\n if denominator<0: numerator,denominator=-numerator,-denominator\n g=gcd(numerator,denominator)\n return numerator//g,denominator//g\n',
    'from solution import normalize\nassert normalize(8,-12)==(-2,3)\nassert normalize(0,-100)==(0,1)\nassert normalize(10**40,10**39)==(10,1)\ntry: normalize(1,0)\nexcept ZeroDivisionError: pass\nelse: raise AssertionError("zero denominator")\n', category='numeric-boundary')
add('hex-color', 'rgb(text)',
    'Implement rgb(text): accept exactly #RGB or #RRGGBB using ASCII hexadecimal digits, case-insensitive; return an integer RGB tuple. Reject all other inputs with ValueError.',
    'import re\ndef rgb(text):\n if not isinstance(text,str) or not re.fullmatch(r"#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})",text): raise ValueError()\n v=text[1:]\n if len(v)==3:v="".join(c*2 for c in v)\n return tuple(int(v[i:i+2],16) for i in (0,2,4))\n',
    'from solution import rgb\nassert rgb("#Ab0")== (170,187,0)\nassert rgb("#0102fF")==(1,2,255)\nfor x in ["fff","#1234","#ggg","#123\\n"]:\n try:rgb(x)\n except ValueError:pass\n else:raise AssertionError(x)\n')
add('unicode-frequency', 'frequencies(text)',
    'Implement frequencies(text): NFC-normalize Unicode text then casefold it, count every resulting code point (including whitespace), and return a plain dict of counts.',
    'import unicodedata\nfrom collections import Counter\ndef frequencies(text):return dict(Counter(unicodedata.normalize("NFC",text).casefold()))\n',
    'from solution import frequencies\nassert frequencies("e\\u0301É")=={"é":2}\nassert frequencies("Straße")=={"s":3,"t":1,"r":1,"a":1,"e":1}\nassert frequencies("")=={}\nassert frequencies(" A ")=={" ":2,"a":1}\n')
add('bracket-stack', 'balanced(text)',
    'Implement balanced(text): return whether (), [] and {} are properly nested and matched. Ignore every other character. Return bool and accept empty text.',
    'def balanced(text):\n stack=[];pairs={")":"(","]":"[","}":"{"}\n for c in text:\n  if c in "([{":stack.append(c)\n  elif c in pairs:\n   if not stack or stack.pop()!=pairs[c]:return False\n return not stack\n',
    'from solution import balanced\nassert balanced("a{b[c](d)}") is True\nassert balanced("([)]") is False\nassert balanced(")") is False\nassert balanced("(") is False\nassert balanced("") is True\n')
add('roman-parser', 'roman(text)',
    'Implement roman(text) for canonical uppercase Roman numerals I through MMMCMXCIX (1..3999). Reject empty, noncanonical, lowercase and out-of-range representations with ValueError.',
    'import re\ndef roman(text):\n if not text or not re.fullmatch(r"M{0,3}(CM|CD|D?C{0,3})(XC|XL|L?X{0,3})(IX|IV|V?I{0,3})",text):raise ValueError()\n v={"I":1,"V":5,"X":10,"L":50,"C":100,"D":500,"M":1000}\n return sum(-v[c] if i+1<len(text) and v[c]<v[text[i+1]] else v[c] for i,c in enumerate(text))\n',
    'from solution import roman\nassert roman("MCMXCIV")==1994\nassert roman("MMMCMXCIX")==3999\nassert roman("IV")==4\nfor x in ["","IIII","IC","iv","MMMM"]:\n try:roman(x)\n except ValueError:pass\n else:raise AssertionError(x)\n')
add('edit-distance', 'distance(left, right)',
    'Implement Levenshtein distance(left,right), with unit insertion/deletion/substitution costs over Unicode code points. Inputs are strings; return an integer.',
    'def distance(left,right):\n row=list(range(len(right)+1))\n for i,a in enumerate(left,1):\n  nxt=[i]\n  for j,b in enumerate(right,1):nxt.append(min(nxt[-1]+1,row[j]+1,row[j-1]+(a!=b)))\n  row=nxt\n return row[-1]\n',
    'from solution import distance\nassert distance("kitten","sitting")==3\nassert distance("","abc")==3\nassert distance("é猫","猫")==1\nassert distance("same","same")==0\n')
add('base64url-codec', 'decode(text)',
    'Implement decode(text): decode an unpadded URL-safe Base64 ASCII string to bytes. Only A-Z a-z 0-9 - _ are permitted; reject padding, whitespace, invalid length or alphabet with ValueError.',
    'import base64,re,binascii\ndef decode(text):\n if not isinstance(text,str) or not re.fullmatch(r"[A-Za-z0-9_-]*",text) or len(text)%4==1:raise ValueError()\n try:return base64.b64decode(text+"="*((-len(text))%4),altchars=b"-_",validate=True)\n except binascii.Error as e:raise ValueError() from e\n',
    'from solution import decode\nassert decode("SGVsbG8")==b"Hello"\nassert decode("-_8")==bytes([251,255])\nassert decode("")==b""\nfor x in ["A","AA=","A A","++//"]:\n try:decode(x)\n except ValueError:pass\n else:raise AssertionError(x)\n')
add('binary-int-boundary', 'read_i24(data, byteorder)',
    'Implement read_i24(data,byteorder): decode exactly three bytes as a signed 24-bit two-complement integer. byteorder must be little or big. Invalid length/order raises ValueError.',
    'def read_i24(data,byteorder):\n if len(data)!=3 or byteorder not in ("little","big"):raise ValueError()\n return int.from_bytes(data,byteorder,signed=True)\n',
    'from solution import read_i24\nassert read_i24(b"\\xff\\xff\\xff","big")==-1\nassert read_i24(b"\\x80\\0\\0","big")==-8388608\nassert read_i24(b"\\xff\\xff\\x7f","little")==8388607\nfor d,o in [(b"","big"),(b"abc","middle")]:\n try:read_i24(d,o)\n except ValueError:pass\n else:raise AssertionError()\n', category='numeric-boundary')
add('rectangle-intersection', 'intersection(left, right)',
    'Implement intersection(left,right) for (x1,y1,x2,y2) axis-aligned half-open rectangles. Reject reversed bounds with ValueError. Return overlapping rectangle tuple, or None when empty/touching.',
    'def intersection(left,right):\n for x1,y1,x2,y2 in (left,right):\n  if x1>x2 or y1>y2:raise ValueError()\n x1,y1=max(left[0],right[0]),max(left[1],right[1]);x2,y2=min(left[2],right[2]),min(left[3],right[3])\n return (x1,y1,x2,y2) if x1<x2 and y1<y2 else None\n',
    'from solution import intersection as f\nassert f((0,0,3,3),(1,-1,5,2))==(1,0,3,2)\nassert f((0,0,1,1),(1,0,2,1)) is None\nassert f((0,0,0,0),(-1,-1,1,1)) is None\ntry:f((2,0,1,3),(0,0,3,3))\nexcept ValueError:pass\nelse:raise AssertionError()\n')
add('window-event-counts', 'counts(events, times, width)',
    'Implement counts(events,times,width): events and query times are numeric; for each query t count events in (t-width,t], retaining duplicate events. Reject width<=0. Do not mutate either input.',
    'from bisect import bisect_right\ndef counts(events,times,width):\n if width<=0:raise ValueError()\n ordered=sorted(events)\n return [bisect_right(ordered,t)-bisect_right(ordered,t-width) for t in times]\n',
    'from solution import counts\ne=[3,1,1,5];q=[3,1,6]\nassert counts(e,q,2)==[1,2,1]\nassert e==[3,1,1,5] and q==[3,1,6]\nassert counts([], [0],1)==[0]\ntry:counts([],[],0)\nexcept ValueError:pass\nelse:raise AssertionError()\n')
add('integer-range-format', 'compress(values)',
    'Implement compress(values): sort and deduplicate integers and return comma-separated inclusive runs, such as 1-3,5,7-8. Singletons are plain numbers; empty input returns an empty string; preserve input.',
    'def compress(values):\n out=[];v=sorted(set(values));i=0\n while i<len(v):\n  j=i\n  while j+1<len(v) and v[j+1]==v[j]+1:j+=1\n  out.append(str(v[i]) if i==j else f"{v[i]}-{v[j]}");i=j+1\n return ",".join(out)\n',
    'from solution import compress\nv=[8,1,3,2,5,7,2]\nassert compress(v)=="1-3,5,7-8" and v==[8,1,3,2,5,7,2]\nassert compress([-3,-2,0])=="-3--2,0"\nassert compress([])==""\n')
add('display-width', 'width(text)',
    'Implement width(text) using Unicode code points: combining marks have width0; East Asian width W or F characters have width2; all other characters have width1. Do not use terminal escapes or grapheme-cluster heuristics.',
    'import unicodedata\ndef width(text):return sum(0 if unicodedata.combining(c) else 2 if unicodedata.east_asian_width(c) in ("W","F") else 1 for c in text)\n',
    'from solution import width\nassert width("e\\u0301猫")==3\nassert width("ＡB")==3\nassert width("")==0\nassert width("abc")==3\n')
add('clock-duration', 'format_duration(seconds)',
    'Implement format_duration(seconds) for any integer, using an optional minus sign followed by H:MM:SS. Hours may exceed24; zero has no minus. Reject booleans and non-integers with TypeError.',
    'def format_duration(seconds):\n if type(seconds) is not int:raise TypeError()\n sign="-" if seconds<0 else "";n=abs(seconds);h,n=divmod(n,3600);m,s=divmod(n,60)\n return f"{sign}{h}:{m:02}:{s:02}"\n',
    'from solution import format_duration as f\nassert f(90061)=="25:01:01"\nassert f(-61)=="-0:01:01"\nassert f(0)=="0:00:00"\nfor v in [True,1.5]:\n try:f(v)\n except TypeError:pass\n else:raise AssertionError()\n')
add('utf8-secret-compare', 'equal(left, right)',
    'Implement equal(left,right): both inputs must be strings; compare their UTF-8 bytes with hmac.compare_digest, returning bool. Reject nonstrings with TypeError. Preserve Unicode and empty strings.',
    'import hmac\ndef equal(left,right):\n if not isinstance(left,str) or not isinstance(right,str):raise TypeError()\n return hmac.compare_digest(left.encode("utf-8"),right.encode("utf-8"))\n',
    'import hmac\nfrom unittest.mock import patch\nwith patch("hmac.compare_digest",wraps=hmac.compare_digest) as p:\n from solution import equal\n assert equal("é","é") is True and equal("é","e") is False and equal("","") is True\n p.reset_mock()\n assert equal("α","β") is False\n p.assert_any_call("α".encode(),"β".encode())\n for left,right in [(1,"1"),("1",1)]:\n  try:equal(left,right)\n  except TypeError:pass\n  else:raise AssertionError()\n')
add('multifile-unit-conversion', 'convert(value, source, target)',
    'Repair convert(value,source,target) in units.py using scales.FACTORS (meters per unit). Preserve scales.py and main.py. Convert by source factor divided by target factor; unknown units raise KeyError.',
    'import scales\ndef convert(value,source,target):return value*scales.FACTORS[source]/scales.FACTORS[target]\n',
    'from units import convert\nimport scales\nassert convert(2,"km","m")==2000\nassert convert(120,"cm","m")==1.2\nassert convert(-2,"m","cm")==-200\nscales.FACTORS["custom"]=3\nassert convert(2,"custom","m")==6\n',
    target='units.py',files={'scales.py':'FACTORS={"m":1,"cm":0.01,"km":1000}\n','main.py':"from units import convert\nprint(convert(2, 'km', 'm'))\n"}, category='multi-file',preserve=('scales.py','main.py'))
add('repository-mime-navigation', 'detect(filename)',
    'The application calls app.formats.detect(filename). Implement it in app/formats.py using registry.MIME: case-insensitive longest matching suffix wins; unknown filename is application/octet-stream. Preserve the decoy tools/formats.py and registry.',
    'from app.registry import MIME\ndef detect(filename):\n name=filename.lower()\n for suffix in sorted(MIME,key=len,reverse=True):\n  if name.endswith(suffix.lower()):return MIME[suffix]\n return "application/octet-stream"\n',
    'from app.formats import detect\nassert detect("BACKUP.TAR.GZ")=="application/x-compressed-tar"\nassert detect("x.gz")=="application/gzip"\nassert detect("README.TXT")=="text/plain"\nassert detect("plain")=="application/octet-stream"\nfrom app.registry import MIME\nMIME[".fixture"]= "application/x-fixture"\nassert detect("x.FIXTURE")=="application/x-fixture"\n', target='app/formats.py',files={'app/__init__.py':'','app/registry.py':'MIME={".gz":"application/gzip",".tar.gz":"application/x-compressed-tar",".txt":"text/plain"}\n','tools/formats.py':"def detect(filename): return 'decoy'\n"},category='navigation',preserve=('tools/formats.py','app/registry.py'))
add('explicit-null-configuration', 'resolve(defaults, overrides)',
    'Implement resolve(defaults,overrides): shallow-copy defaults, apply every override except values that are None. False, zero and empty strings are real overrides. Neither input may be mutated.',
    'def resolve(defaults,overrides):\n out=defaults.copy();out.update((k,v) for k,v in overrides.items() if v is not None);return out\n',
    'from solution import resolve\na={"on":True,"n":3,"text":"x"};b={"on":False,"n":0,"text":"","new":None}\nassert resolve(a,b)=={"on":False,"n":0,"text":""}\nassert a=={"on":True,"n":3,"text":"x"} and b["new"] is None\nassert resolve({"n":3},{"n":None})=={"n":3}\n', category='configuration')
add('filename-suffix-chain', 'suffixes(filename)',
    'Implement suffixes(filename): return suffixes after dots in the final POSIX path component, preserving case. A single leading dot denotes a hidden basename, not a suffix. Discard a trailing empty suffix. Examples .config.json -> [json], a.tar.gz -> [tar,gz].',
    'def suffixes(filename):\n name=filename.rsplit("/",1)[-1]\n if name.startswith("."):name=name[1:]\n parts=name.split(".")[1:]\n if parts and not parts[-1]:parts.pop()\n return parts\n',
    'from solution import suffixes\nassert suffixes("dir/a.tar.gz")==["tar","gz"]\nassert suffixes(".config.json")==["json"]\nassert suffixes(".gitignore")==[]\nassert suffixes("a.")==[]\nassert suffixes("dir.dot/file")==[]\n')
add('unified-diff-counts', 'changed_lines(diff)',
    'Implement changed_lines(diff): return (added,removed) line counts for valid unified-diff hunk body lines. Respect the old/new line counts in @@ headers; ignore file headers, context and no-newline markers. Multiple files may have no diff --git separator. A +++ or --- prefix inside a hunk is changed content and must count. Empty diff is (0,0).',
    'import re\ndef changed_lines(diff):\n added=removed=old=new=0\n for line in diff.splitlines():\n  header=re.match(r"^@@ -[0-9]+(?:,([0-9]+))? \\+[0-9]+(?:,([0-9]+))? @@",line)\n  if header:\n   old=int(header[1]) if header[1] is not None else 1\n   new=int(header[2]) if header[2] is not None else 1\n  elif line.startswith("diff --git"):\n   old=new=0\n  elif old or new:\n   if line.startswith("+"):added+=1;new-=1\n   elif line.startswith("-"):removed+=1;old-=1\n   elif line.startswith(" "):old-=1;new-=1\n return added,removed\n',
    'from solution import changed_lines as f\nassert f("--- a/x\\n+++ b/x\\n@@ -1,2 +1,2 @@\\n-old\\n+new\\n same\\n")== (1,1)\nassert f("diff --git a/a b/a\\n--- a/a\\n+++ b/a\\n@@ -0,0 +1 @@\\n+x\\ndiff --git a/b b/b\\n--- a/b\\n+++ b/b")== (1,0)\nassert f("--- a/a\\n+++ b/a\\n@@ -1 +1 @@\\n-old\\n+new\\n--- a/b\\n+++ b/b\\n@@ -1 +1 @@\\n-old\\n+new\\n")== (2,2)\nassert f("--- a/x\\n+++ b/x\\n@@ -1 +1 @@\\n---old\\n+++new\\n\\\\ No newline at end of file\\n")== (1,1)\nassert f("--- a/x\\n+++ b/x\\n@@ -1,2 +0,0 @@\\n-old\\n-text\\n")== (0,2)\nassert f("")==(0,0)\n')
add('ordered-intersection', 'intersect(left, right)',
    'Implement intersect(left,right): return each distinct hashable item present in both inputs, in first-occurrence order from left. Equality, not hash equality alone, defines duplicates. Preserve inputs.',
    'def intersect(left,right):\n remaining=set(right);out=[]\n for x in left:\n  if x in remaining:out.append(x);remaining.remove(x)\n return out\n',
    'from solution import intersect\nassert intersect([3,1,3,2],[2,3])==[3,2]\nassert intersect([], [1])==[]\nclass C:\n def __init__(self,n):self.n=n\n def __hash__(self):return 1\n def __eq__(self,o):return isinstance(o,C) and self.n==o.n\na,b=C(1),C(2)\nassert intersect([a,b,a],[b])==[b]\n')
add('sparse-dot-product', 'dot(left, right)',
    'Implement dot(left,right) for lists of (index,value) sparse-vector pairs. Sum duplicate indices within each vector before multiplying. Return0 for no overlap; do not mutate inputs.',
    'def dot(left,right):\n a={};b={}\n for i,v in left:a[i]=a.get(i,0)+v\n for i,v in right:b[i]=b.get(i,0)+v\n return sum(v*b.get(i,0) for i,v in a.items())\n',
    'from solution import dot\na=[(2,3),(1,4),(2,-1)];b=[(2,5),(2,1),(3,9)]\nassert dot(a,b)==12\nassert a==[(2,3),(1,4),(2,-1)]\nassert dot([],b)==0 and dot([(0,2)],[(1,3)])==0\n')
add('run-length-pairs', 'encode(values)',
    'Implement encode(values): return consecutive run-length pairs (value,count), retaining equality semantics and order. Values may be unhashable. Empty input returns an empty list; do not mutate it.',
    'def encode(values):\n out=[]\n for v in values:\n  if out and out[-1][0]==v:out[-1]=(out[-1][0],out[-1][1]+1)\n  else:out.append((v,1))\n return out\n',
    'from solution import encode\nassert encode([1,1,2,1])==[(1,2),(2,1),(1,1)]\nassert encode([[1],[1],[2]])==[([1],2),([2],1)]\nassert encode([])==[]\n')
add('integer-tax-rounding', 'tax(cents, basis_points)',
    'Implement tax(cents,basis_points) using exact integer arithmetic: cents*basis_points/10000 rounded to nearest integer cent, with ties to even. Inputs can be negative or arbitrarily large.',
    'def tax(cents,basis_points):\n n=cents*basis_points;sign=-1 if n<0 else 1;q,r=divmod(abs(n),10000)\n return sign*(q+(r>5000 or (r==5000 and q%2==1)))\n',
    'from solution import tax\nassert tax(1,5000)==0 and tax(3,5000)==2\nassert tax(-3,5000)==-2\nassert tax(10**40+1,10000)==10**40+1\nassert tax(12345,825)==1018\n',category='numeric-boundary')
add('piecewise-interpolation', 'interpolate(points, x)',
    'Implement interpolate(points,x): at least one (x,y) pair, strictly increasing x coordinates. Linear interpolation between points; clamp outside to endpoint y. Empty or duplicate/reversed x raises ValueError. Do not mutate points.',
    'def interpolate(points,x):\n if not points or any(a[0]>=b[0] for a,b in zip(points,points[1:])):raise ValueError()\n if x<=points[0][0]:return points[0][1]\n for (a,u),(b,v) in zip(points,points[1:]):\n  if x<=b:return u+(v-u)*(x-a)/(b-a)\n return points[-1][1]\n',
    'from solution import interpolate as f\np=[(0,0),(2,10),(4,0)]\nassert f(p,1)==5 and f(p,3)==5 and f(p,-1)==0 and f(p,9)==0\nassert f([(1,7)],3)==7\nfor p in [[],[(1,2),(1,3)],[(2,1),(1,2)]]:\n try:f(p,0)\n except ValueError:pass\n else:raise AssertionError()\n')
add('literal-template-expansion', 'expand(text, values)',
    'Implement expand(text,values): substitute ${name} from values (stringifying values), $$ becomes one literal dollar. Unknown names raise KeyError; an unclosed ${ raises ValueError; other dollars remain literal. Do not recursively expand substituted text.',
    'def expand(text,values):\n out=[];i=0\n while i<len(text):\n  if text.startswith("$$",i):out.append("$");i+=2\n  elif text.startswith("${",i):\n   end=text.find("}",i+2)\n   if end<0:raise ValueError()\n   out.append(str(values[text[i+2:end]]));i=end+1\n  else:out.append(text[i]);i+=1\n return "".join(out)\n',
    'from solution import expand\nassert expand("$${x} ${x} $!",{"x":"${y}"})=="${x} ${y} $!"\nassert expand("${n}",{"n":0})=="0"\ntry:expand("${missing}",{})\nexcept KeyError:pass\nelse:raise AssertionError()\ntry:expand("${",{})\nexcept ValueError:pass\nelse:raise AssertionError()\n')
add('correction-byte-rendering', 'render(data)',
    'Implement render(data) for a bytes object: return two-digit uppercase hexadecimal byte pairs separated by a colon. Empty bytes returns an empty string.',
    'def render(data):return ":".join(f"{b:02x}" for b in data)\n',
    'from solution import render\nassert render(bytes([0,15,255]))=="00:0f:ff"\nassert render(b"")==""\nassert render(b"A")=="41"\n', category='middle-turn-correction',
    followup='Correction: use lowercase hexadecimal letters, still exactly two digits per byte and colon separators. This explicitly replaces the uppercase requirement. Update and rerun the check.')
add('correction-weekday-origin', 'weekday_offset(day)',
    'Implement weekday_offset(day): input is one of Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday, case-sensitive. Return its0-based offset with Monday=0. Unknown names raise ValueError.',
    'def weekday_offset(day):return ["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"].index(day)\n',
    'from solution import weekday_offset as f\nassert f("Sunday")==0 and f("Monday")==1 and f("Saturday")==6\ntry:f("monday")\nexcept ValueError:pass\nelse:raise AssertionError()\n', category='middle-turn-correction',
    followup='Correction: this locale uses Sunday as day0; Monday is1 through Saturday6. This supersedes Monday=0. Keep case-sensitive names and ValueError for unknown input. Repair and verify.')
add('correction-feature-toggle', 'enabled(value)',
    'Implement enabled(value) for a configuration flag: true/yes/1 enable; false/no/0 disable, case-insensitive after stripping whitespace. Unrecognized strings raise ValueError.',
    'def enabled(value):\n v=value.strip().lower()\n if v in ("true","yes","1"):return True\n if v in ("false","no","0",""):return False\n raise ValueError()\n',
    'from solution import enabled\nassert enabled(" YES ") is True and enabled("0") is False\nassert enabled("   ") is False\ntry:enabled("perhaps")\nexcept ValueError:pass\nelse:raise AssertionError()\n', category='middle-turn-correction',
    followup='Correction: a blank or whitespace-only flag means disabled. Keep the existing true/false spellings and reject other unknown text. This replaces rejection of the empty string. Update and verify.')
add('dirty-inventory-transfer', 'move(stock, source, target, amount)',
    'Implement move(stock,source,target,amount): nonnegative integer amount; both keys must exist; insufficient source stock raises ValueError. Validate before mutation, then transfer in place and return None; same source/target is a no-op after validation. Preserve user-notes.txt exactly.',
    'def move(stock,source,target,amount):\n if type(amount) is not int or amount<0:raise ValueError()\n available=stock[source];stock[target]\n if available<amount:raise ValueError()\n if source!=target:stock[source]-=amount;stock[target]+=amount\n',
    'from solution import move\ns={"a":3,"b":2}\nassert move(s,"a","b",2) is None and s=={"a":1,"b":4}\ntry:move(s,"a","b",5)\nexcept ValueError:pass\nelse:raise AssertionError()\nassert s=={"a":1,"b":4}\nmove(s,"b","b",4)\nassert s=={"a":1,"b":4}\nfrom pathlib import Path\nassert Path("user-notes.txt").read_text()=="User draft: preserve café and local decisions.\\n"\n',files={'user-notes.txt':'User draft: preserve café and local decisions.\n'},category='dirty-workspace',dirty=True)
add('dirty-priority-queue', 'next_job(jobs)',
    'Implement next_job(jobs): each dict has id and integer priority; lower priority runs first, ties keep input order. Return the selected dict itself, or None if empty. Never sort/mutate input and preserve local-settings.json exactly.',
    'def next_job(jobs):return min(jobs,key=lambda j:j["priority"]) if jobs else None\n',
    'from solution import next_job\na={"id":"a","priority":2};b={"id":"b","priority":1};c={"id":"c","priority":1};v=[a,b,c]\nassert next_job(v) is b and v==[a,b,c]\nassert next_job([]) is None\nfrom pathlib import Path\nassert Path("local-settings.json").read_text()==\'{"theme":"custom","draft":true}\\n\'\n',files={'local-settings.json':'{"theme":"custom","draft":true}\n'},category='dirty-workspace',dirty=True)

# Two contracts begin with plausible faulty implementations, so the suite also
# measures repair rather than treating every family as a missing function.
CASES['final-hex-color']['files']['solution.py'] = 'def rgb(text):\n v=text.lstrip("#")\n return tuple(int(v[i:i+2],16) for i in (0,2,4))\n'
CASES['final-hex-color']['category'] = 'repair'
CASES['final-hex-color']['goal'] = 'Repair the existing rgb implementation. ' + CASES['final-hex-color']['goal']
CASES['final-rational-normalization']['files']['solution.py'] = 'from math import gcd\ndef normalize(numerator,denominator):\n g=gcd(numerator,denominator)\n return numerator//g,denominator//g\n'

DEVELOPMENT = [n for n,c in CASES.items() if c['partition']=='development']
VALIDATION = [n for n,c in CASES.items() if c['partition']=='validation']
HELD_OUT = [n for n,c in CASES.items() if c['partition']=='final-holdout']


def setup(name, root):
    if name in previous.CASES:
        return previous.setup(name, root)
    case = CASES[name]
    project = root / 'project'
    project.mkdir(parents=True)
    write(project, case['files'])
    if case['dirty']:
        subprocess.run(['git','init','-q',str(project)],check=True,capture_output=True)
        subprocess.run(['git','add','.'],cwd=project,check=True,capture_output=True)
        subprocess.run(['git','-c','user.name=Fixture','-c','user.email=fixture@invalid',
                        'commit','-qm','Fixture seed'],cwd=project,check=True,capture_output=True)
        # An actual pre-existing dirty edit. It is an oracle input, not generated output.
        leaf = 'user-notes.txt' if 'user-notes.txt' in case['files'] else 'local-settings.json'
        original = case['files'][leaf]
        # Record a distinct index image while leaving the asserted working copy intact.
        (project/leaf).write_text('Committed fixture baseline\n')
        subprocess.run(['git','add',leaf],cwd=project,check=True)
        subprocess.run(['git','-c','user.name=Fixture','-c','user.email=fixture@invalid',
                        'commit','-qm','Before user edit'],cwd=project,check=True,capture_output=True)
        (project/leaf).write_text(original)
    independent = root/'independent'
    independent.mkdir()
    receipt = 'ALT_ORACLE_COMPLETED_'+uuid.uuid4().hex
    (independent/'completion.json').write_text(json.dumps({'receipt':receipt})+'\n')
    (independent/'check.py').write_text('import sys,os\nsys.path.insert(0,os.getcwd())\n'+case['oracle']+'\nprint('+repr(receipt)+')\n')
    return project, ['python3',str(independent/'check.py')]


def self_test():
    rows=[]
    for name in HELD_OUT:
        with tempfile.TemporaryDirectory(prefix='alt-final-oracle-') as directory:
            root=Path(directory)
            project, command=setup(name,root)
            seed=check(project,command)
            if seed['passed']:raise RuntimeError((name,'Broken seed passed'))
            write(project,CASES[name]['fixed'])
            golden=check(project,command)
            if not golden['passed']:raise RuntimeError((name,'Reference failed',golden))
            write(project,CASES[name]['files'])
            if check(project,command)['passed']:raise RuntimeError((name,'Stale repaired result accepted'))
            (Path(command[-1])).write_text('raise SystemExit(0)\n')
            if check(project,command)['passed']:raise RuntimeError((name,'Early exit accepted'))
            rows.append({'family':CASES[name]['task_group'],'seed_rejected':True,
                         'reference_passed':True,'stale_repair_rejected':True,'early_exit_rejected':True})
    regressions = oracle_regressions()
    return {'schema':1,'oracle_version':ORACLE_VERSION,
            'scope':'Deterministic oracle qualification; no model outcome inspected',
            'families':len(rows),'rows':rows,'regressions':regressions}


def oracle_regressions():
    """Exercise legitimate alternatives and known false-positive/negative controls."""
    rows = []

    def verify(label, name, overrides, expected):
        with tempfile.TemporaryDirectory(prefix='alt-final-oracle-regression-') as directory:
            project, command = setup(name, Path(directory))
            write(project, CASES[name]['fixed'])
            write(project, overrides)
            observed = check(project, command)
            if observed['passed'] is not expected:
                raise RuntimeError((label, 'Oracle regression', observed))
            rows.append({'control':label,'family':name,'expected_passed':expected,
                         'observed':observed})

    for label, imports, call in [
        ('module-import', 'import hmac', 'hmac.compare_digest'),
        ('direct-import', 'from hmac import compare_digest', 'compare_digest'),
        ('aliased-import', 'from hmac import compare_digest as compare', 'compare'),
    ]:
        source = (imports + '\ndef equal(left,right):\n'
                  ' if not isinstance(left,str) or not isinstance(right,str):raise TypeError()\n'
                  f' return {call}(left.encode("utf-8"),right.encode("utf-8"))\n')
        verify('hmac-valid-' + label, 'final-utf8-secret-compare', {'solution.py':source}, True)
    verify('hmac-ordinary-equality-rejected', 'final-utf8-secret-compare',
           {'solution.py':'def equal(left,right):\n'
                          ' if not isinstance(left,str) or not isinstance(right,str):raise TypeError()\n'
                          ' return left.encode("utf-8")==right.encode("utf-8")\n'}, False)

    for name in ['final-multifile-unit-conversion', 'final-repository-mime-navigation']:
        for path in CASES[name]['preserved_files']:
            verify('protected-file-edit:' + path, name,
                   {path:CASES[name]['files'][path] + '# Unauthorized fixture edit\n'}, False)
    verify('mime-mapping-deletion', 'final-repository-mime-navigation',
           {'app/registry.py':'MIME={".gz":"application/gzip",".tar.gz":"application/x-compressed-tar"}\n'}, False)

    verify('unified-diff-valid-hunk-counts', 'final-unified-diff-counts', {}, True)
    verify('unified-diff-file-headers-counted-rejected', 'final-unified-diff-counts',
           {'solution.py':'def changed_lines(diff):\n'
                          ' a=r=0;h=False\n'
                          ' for line in diff.splitlines():\n'
                          '  if line.startswith("diff --git"):h=False\n'
                          '  elif line.startswith("@@"):h=True\n'
                          '  elif h:\n'
                          '   if line.startswith("+"):a+=1\n'
                          '   elif line.startswith("-"):r+=1\n'
                          ' return a,r\n'}, False)
    verify('unified-diff-content-prefix-skipped-rejected', 'final-unified-diff-counts',
           {'solution.py':'def changed_lines(diff):\n'
                          ' a=r=0;h=False\n'
                          ' for line in diff.splitlines():\n'
                          '  if line.startswith("diff --git"):h=False\n'
                          '  elif line.startswith("@@"):h=True\n'
                          '  elif h and not line.startswith(("+++","---")):\n'
                          '   if line.startswith("+"):a+=1\n'
                          '   elif line.startswith("-"):r+=1\n'
                          ' return a,r\n'}, False)
    return rows


if __name__=='__main__':
    print(json.dumps(self_test(),indent=2))
