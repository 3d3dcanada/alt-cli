#!/usr/bin/env python3
"""Revision 5: historical blind spots become development cases; fresh test families.

Never use HELD_OUT or its reference repairs in prompt optimization or training.
The v4 module remains byte-for-byte unchanged for historical replay.
"""
import copy
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import uuid
from pathlib import Path
if sys.flags.optimize:raise RuntimeError('Run without Python optimization; evidence assertions must remain enabled')
import acceptance_projects as historical
from acceptance_projects import check, oracle_inputs, assertion_script, write

ORACLE_VERSION = 5
CASES = copy.deepcopy(historical.CASES)
for name, case in CASES.items():
    case['partition'] = 'development'
    case['task_group'] = name.removeprefix('heldout-')

CASES['heldout-deduplicate']['oracle'] += '''
class Collision:
 def __init__(self, value): self.value=value
 def __hash__(self): return 1
 def __eq__(self, other): return isinstance(other,Collision) and self.value==other.value
assert unique([None,None,False,False],lambda x:x)==[None,False]
values=[{'id':1},{'id':2},{'id':1}]
assert unique(values,lambda x:Collision(x['id']))==values[:2]
'''
CASES['heldout-toposort']['oracle'] += '''
assert order({})==[]
r=order({'alone':[],'app':['db'],'db':[],'other':[]})
assert len(r)==4 and set(r)=={'alone','app','db','other'} and r.index('db')<r.index('app')
try: order({'self':['self']})
except ValueError: pass
else: raise AssertionError('self cycle')
'''
CASES['heldout-date']['oracle'] += '''
assert utc_timestamp('2024-02-29T23:59:59Z')==1709251199
assert utc_timestamp('2023-12-31T23:30:00-02:00')==1704072600
assert utc_timestamp('1970-01-01T00:00:00Z')==0
assert utc_timestamp('1969-12-31T23:59:59Z')==-1
'''
CASES['heldout-atomic-save']['oracle'] += '''
import os
with tempfile.TemporaryDirectory() as d:
 p=Path(d)/'new'; seen=[]
 original_replace=os.replace
 def replace(src,dst):
  assert Path(src).parent.resolve()==p.parent.resolve()
  assert seen==['fsync']
  return original_replace(src,dst)
 with patch('os.fsync',side_effect=lambda fd:seen.append('fsync')),patch('os.replace',side_effect=replace):
  save(p,'new ☃')
 assert p.read_text()=='new ☃' and len(list(Path(d).iterdir()))==1
'''

def add(name, goal, seed, fixed, oracle):
    CASES[name] = {'goal':goal, 'files':{'solution.py':seed},
                   'fixed':{'solution.py':fixed}, 'oracle':oracle,
                   'partition':'held-out', 'task_group':name.removeprefix('sealed-')}

add('sealed-intervals',
    'Implement merge(intervals) for integer half-open intervals (start,end). Reject reversed intervals with ValueError, discard empty intervals, merge overlapping or touching intervals, return sorted tuples, and do not mutate input.',
    'def merge(intervals): return intervals\n',
    '''def merge(intervals):
 items=[]
 for a,b in intervals:
  if a>b: raise ValueError('reversed interval')
  if a<b: items.append((a,b))
 out=[]
 for a,b in sorted(items):
  if out and a<=out[-1][1]: out[-1]=(out[-1][0],max(b,out[-1][1]))
  else: out.append((a,b))
 return out
''',
    '''from solution import merge
source=[(8,10),(1,3),(3,7),(2,4),(9,9)]; original=source.copy()
assert merge(source)==[(1,7),(8,10)] and source==original
assert merge([])==[] and merge([(0,0)])==[]
assert merge([(-4,-2),(-3,5),(1,2)])==[(-4,5)]
try: merge([(3,2)])
except ValueError: pass
else: raise AssertionError('reversed interval')
''')
add('sealed-stream-lines',
    'Implement lines(chunks): an iterator of decoded UTF-8 lines from byte chunks. Chunk boundaries may split multibyte characters or CRLF. Strip LF and an immediately preceding CR, retain empty lines, and emit a nonempty final unterminated line. Invalid UTF-8 must raise UnicodeDecodeError.',
    'def lines(chunks): return []\n',
    '''import codecs
def lines(chunks):
 decoder=codecs.getincrementaldecoder('utf-8')(); pending=''
 for chunk in chunks:
  pending+=decoder.decode(chunk)
  while '\\n' in pending:
   line,pending=pending.split('\\n',1)
   yield line[:-1] if line.endswith('\\r') else line
 pending+=decoder.decode(b'',final=True)
 if pending: yield pending
''',
    '''from solution import lines
data='☃\\r\\n\\ncedar\\nlast'.encode()
assert list(lines([bytes([b]) for b in data]))==['☃','','cedar','last']
assert list(lines([]))==[] and list(lines([b'\\n']))==['']
assert list(lines([b'x\\r']))==['x\\r']
try: list(lines([b'\\xe2',b'\\x28']))
except UnicodeDecodeError: pass
else: raise AssertionError('invalid UTF-8')
''')
add('sealed-cache-expiry',
    'Implement Cache(clock) with put(key,value,ttl) and get(key,default=None). ttl must be nonnegative and is measured with the supplied clock. A value expires when clock() >= insertion_time+ttl. Preserve false/None values and overwrite keys; do not use wall-clock time.',
    'class Cache:\n def __init__(self,clock): pass\n def put(self,key,value,ttl): pass\n def get(self,key,default=None): return default\n',
    '''class Cache:
 def __init__(self,clock): self.clock=clock; self.items={}
 def put(self,key,value,ttl):
  if ttl<0: raise ValueError('ttl')
  self.items[key]=(value,self.clock()+ttl)
 def get(self,key,default=None):
  if key not in self.items: return default
  value,expiry=self.items[key]
  if self.clock()>=expiry:
   del self.items[key]; return default
  return value
''',
    '''from solution import Cache
now=[10]; c=Cache(lambda:now[0]); c.put('x',False,2); c.put('n',None,5)
assert c.get('x','absent') is False and c.get('n','absent') is None
now[0]=12; assert c.get('x','absent')=='absent'
c.put('x',7,3); now[0]=14; assert c.get('x')==7
now[0]=15; assert c.get('x','absent')=='absent' and c.get('n','absent')=='absent'
c.put('z',1,0); assert c.get('z','absent')=='absent'
try: c.put('bad',1,-1)
except ValueError: pass
else: raise AssertionError('negative ttl')
''')
add('sealed-percent-decoding',
    'Implement decode(text) for percent-encoded UTF-8: decode each %HH byte, keep + as a literal plus, allow literal Unicode, reject malformed percent escapes with ValueError and invalid encoded UTF-8 with UnicodeDecodeError.',
    'def decode(text): return text\n',
    '''def decode(text):
 data=bytearray(); i=0
 while i<len(text):
  if text[i]=='%':
   pair=text[i+1:i+3]
   if len(pair)!=2 or any(c not in '0123456789abcdefABCDEF' for c in pair): raise ValueError('escape')
   data.append(int(pair,16)); i+=3
  else: data.extend(text[i].encode('utf-8')); i+=1
 return data.decode('utf-8')
''',
    '''from solution import decode
assert decode('A%20B+C')=='A B+C' and decode('%E2%98%83')=='☃'
assert decode('café%2fok')=='café/ok' and decode('%25')=='%'
for text in ['%','%2','%GG','%+1']:
 try: decode(text)
 except ValueError: pass
 else: raise AssertionError(text)
try: decode('%FF')
except UnicodeDecodeError: pass
else: raise AssertionError('invalid encoded UTF-8')
''')

DEVELOPMENT = [name for name,case in CASES.items() if case['partition']=='development']
HELD_OUT = [name for name,case in CASES.items() if case['partition']=='held-out']

def setup(name, root):
    if name in ('rust-feature','javascript-config'):
        return historical.setup(name, root)
    case=CASES[name]; project=root/'project'; project.mkdir(parents=True)
    write(project,case['files']); oracle=root/'independent'; oracle.mkdir()
    receipt='ALT_ORACLE_COMPLETED_'+uuid.uuid4().hex
    (oracle/'completion.json').write_text(json.dumps({'receipt':receipt})+'\n')
    (oracle/'check.py').write_text('import sys,os\nsys.path.insert(0,os.getcwd())\n'+case['oracle']+'\nprint('+repr(receipt)+')\n')
    return project, ['python3',str(oracle/'check.py')]

def self_test():
    rows=[]
    weak={
        'heldout-deduplicate':'def unique(values,key):\n seen=set();out=[]\n for v in values:\n  h=hash(key(v))\n  if h not in seen:seen.add(h);out.append(v)\n return out\n',
        'heldout-date':'from datetime import datetime\ndef utc_timestamp(text):\n d=datetime.fromisoformat(text.replace("Z","+00:00"))\n if d.tzinfo is None:raise ValueError("timezone required")\n return 1704067200\n',
        'heldout-toposort':CASES['heldout-toposort']['fixed']['solution.py'].replace('for n in dependencies:visit(n)','for n in dependencies:\n  if dependencies[n]:visit(n)'),
        'heldout-atomic-save':CASES['heldout-atomic-save']['fixed']['solution.py'].replace('os.fsync(f.fileno())','None'),
    }
    for name,case in CASES.items():
        with tempfile.TemporaryDirectory(prefix='alt-v5-') as d:
            root=Path(d); project,command=setup(name,root); inputs=oracle_inputs(root)
            assert not check(project,command)['passed'], (name,'broken seed accepted')
            write(project,case['fixed'])
            assert check(project,command)['passed'], (name,'reference rejected',check(project,command))
            if name in weak:
                write(project,{'solution.py':weak[name]});assert not check(project,command)['passed'], (name,'weak repair accepted');write(project,case['fixed'])
            wrapper=root/'wrapper.py'; wrapper.write_text(assertion_script(root,command,inputs))
            report=root/'report.json'; env={**os.environ,'ALT_CHECK_REPORT':str(report)}
            assert subprocess.run(['python3',str(wrapper)],cwd=project,env=env,capture_output=True).returncode==0
            # Re-running observes changed source; a previously passing receipt cannot win.
            write(project,case['files'])
            assert subprocess.run(['python3',str(wrapper)],cwd=project,env=env,capture_output=True).returncode!=0
            write(project,case['fixed'])
            Path(command[-1]).write_text('import sys; sys.exit(0)\n')
            assert not check(project,command)['passed']
            assert subprocess.run(['python3',str(wrapper)],cwd=project,env=env,capture_output=True).returncode!=0
            rows.append({'case':name,'seed_rejected':True,'reference_passed':True,'weak_repair_rejected':True if name in weak else None,'stale_source_rejected':True,'changed_oracle_rejected':True,'early_exit_rejected':True})
    print(json.dumps({'oracle_version':ORACLE_VERSION,'tasks':rows},indent=2))

if __name__=='__main__': self_test()
