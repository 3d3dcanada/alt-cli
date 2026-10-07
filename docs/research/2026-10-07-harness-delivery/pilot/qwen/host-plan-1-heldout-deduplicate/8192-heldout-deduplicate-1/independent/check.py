import sys,os
sys.path.insert(0,os.getcwd())
from solution import unique
a=[{"id":2,"v":"a"},{"id":1},{"id":2,"v":"b"}]
assert unique(a,lambda x:x["id"])==a[:2]
assert unique([],str)==[]

class Collision:
 def __init__(self, value): self.value=value
 def __hash__(self): return 1
 def __eq__(self, other): return isinstance(other,Collision) and self.value==other.value
assert unique([None,None,False,False],lambda x:x)==[None,False]
values=[{'id':1},{'id':2},{'id':1}]
assert unique(values,lambda x:Collision(x['id']))==values[:2]

print('ALT_ORACLE_COMPLETED_d4234358ef474b158ffd8a5d5fd7ccd3')
