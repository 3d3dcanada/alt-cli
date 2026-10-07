import sys,os
sys.path.insert(0,os.getcwd())
from solution import paginate
assert paginate([1,2,3,4,5],2,2)=={"items":[3,4],"total":5,"pages":3}
assert paginate([],1,5)=={"items":[],"total":0,"pages":0}
assert paginate([1],9,1)["items"]==[]
for p,s in [(0,1),(1,0),(True,2),(1,1.5)]:
 try:paginate([],p,s)
 except ValueError:pass
 else:raise AssertionError((p,s))

print('ALT_ORACLE_COMPLETED_7838dfda93724dd1a173bf5d49b8faaa')
