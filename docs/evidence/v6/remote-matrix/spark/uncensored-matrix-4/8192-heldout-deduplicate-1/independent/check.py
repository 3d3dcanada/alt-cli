import sys,os
sys.path.insert(0,os.getcwd())
from solution import unique
a=[{"id":2,"v":"a"},{"id":1},{"id":2,"v":"b"}]
assert unique(a,lambda x:x["id"])==a[:2]
assert unique([],str)==[]

print('ALT_ORACLE_COMPLETED_7693bd9d4b7143eeb2232061c741329f')
