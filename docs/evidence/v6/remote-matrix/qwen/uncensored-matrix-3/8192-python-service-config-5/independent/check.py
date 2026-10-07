import sys,os
sys.path.insert(0,os.getcwd())
from solution import endpoint
assert endpoint({})=="http://127.0.0.1:8080/health"
assert endpoint({"HOST":"::1","PORT":"3000"})=="http://[::1]:3000/health"
for p in ["0","65536","x"]:
 try:endpoint({"PORT":p})
 except ValueError:pass
 else:raise AssertionError(p)

print('ALT_ORACLE_COMPLETED_d2dc71fe01e5486a98c1d7209c6123fc')
