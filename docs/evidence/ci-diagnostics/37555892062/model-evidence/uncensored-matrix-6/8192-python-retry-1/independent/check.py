import sys,os
sys.path.insert(0,os.getcwd())
from solution import retry
calls=[]
def flaky():
 calls.append(1)
 if len(calls)<3:raise OSError("temporary")
 return 42
assert retry(flaky,3)==42 and len(calls)==3
def bad():raise RuntimeError("permanent")
try:retry(bad,9)
except RuntimeError:pass
else:raise AssertionError()
try:retry(lambda:None,0)
except ValueError:pass
else:raise AssertionError()

print('ALT_ORACLE_COMPLETED_90ea2f2070944df28be40f2618927c62')
