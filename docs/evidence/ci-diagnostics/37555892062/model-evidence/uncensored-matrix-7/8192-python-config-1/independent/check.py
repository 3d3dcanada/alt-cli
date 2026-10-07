import sys,os
sys.path.insert(0,os.getcwd())
from solution import load
import json,tempfile,pathlib
with tempfile.TemporaryDirectory() as d:
 p=pathlib.Path(d)/"config.json"
 for v,expect in [({}, {"host":"localhost","port":8080}),({"host":"x","port":65535},{"host":"x","port":65535})]:
  p.write_text(json.dumps(v));assert load(p)==expect
 for v in [{"port":True},{"port":0},{"port":"80"},{"host":""}]:
  p.write_text(json.dumps(v))
  try:load(p)
  except ValueError:pass
  else:raise AssertionError(v)

print('ALT_ORACLE_COMPLETED_fde0e1b745dc4fb2a3c9029a09ad758f')
