import sys,os
sys.path.insert(0,os.getcwd())
from solution import Cache
now=[10]; c=Cache(lambda:now[0]); c.put('x',False,2); c.put('n',None,5)
assert c.get('x','absent') is False and c.get('n','absent') is None
now[0]=12; assert c.get('x','absent')=='absent'
c.put('x',7,3); now[0]=14; assert c.get('x')==7
now[0]=15; assert c.get('x','absent')=='absent' and c.get('n','absent')=='absent'
c.put('z',1,0); assert c.get('z','absent')=='absent'
try: c.put('bad',1,-1)
except ValueError: pass
else: raise AssertionError('negative ttl')

print('ALT_ORACLE_COMPLETED_2cbb2737b29140a184cc2d0bd969f067')
