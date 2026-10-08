import sys,os
sys.path.insert(0,os.getcwd())
from pricing import discounted
from orders import total
assert discounted(200,10)==180
assert abs(total(200,10,.1)-198)<1e-9
assert total(30,0,0)==30
assert discounted(150,100)==0
assert total(150,100,.2)==0
assert abs(total(150,20,.2)-144)<1e-9

print('ALT_ORACLE_COMPLETED_585628ad668d4a2d953faf5eb48705e5')
