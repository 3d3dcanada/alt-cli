import sys,os
sys.path.insert(0,os.getcwd())
import solution
from vendor import math
assert solution.compute([2,3])==5
math.total=lambda values,strict: (values,strict)
import importlib;importlib.reload(solution)
assert solution.compute([7])==([7],True)
