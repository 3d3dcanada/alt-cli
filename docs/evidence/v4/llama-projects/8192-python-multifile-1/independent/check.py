import sys,os
sys.path.insert(0,os.getcwd())
from pricing import discounted
from orders import total
assert discounted(200,10)==180
assert abs(total(200,10,.1)-198)<1e-9
assert total(30,0,0)==30
