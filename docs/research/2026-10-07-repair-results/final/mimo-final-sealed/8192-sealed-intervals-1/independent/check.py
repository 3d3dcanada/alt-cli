import sys,os
sys.path.insert(0,os.getcwd())
from solution import merge
source=[(8,10),(1,3),(3,7),(2,4),(9,9)]; original=source.copy()
assert merge(source)==[(1,7),(8,10)] and source==original
assert merge([])==[] and merge([(0,0)])==[]
assert merge([(-4,-2),(-3,5),(1,2)])==[(-4,5)]
try: merge([(3,2)])
except ValueError: pass
else: raise AssertionError('reversed interval')

print('ALT_ORACLE_COMPLETED_f7a54cc9cecc4c8e830102e038e7b3bb')
