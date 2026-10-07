import sys,os
sys.path.insert(0,os.getcwd())
from solution import order
r=order({"app":["db","web"],"web":["db"]});assert set(r)=={"app","db","web"} and r.index("db")<r.index("web")<r.index("app")
try:order({"a":["b"],"b":["a"]})
except ValueError:pass
else:raise AssertionError("cycle")

print('ALT_ORACLE_COMPLETED_d79b5c04f5584d22b3395bf49f59a382')
