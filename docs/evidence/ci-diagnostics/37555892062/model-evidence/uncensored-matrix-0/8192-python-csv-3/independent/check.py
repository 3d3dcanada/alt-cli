import sys,os
sys.path.insert(0,os.getcwd())
from solution import totals
assert totals('name,amount\n"Cedar, Inc",0.10\n"Cedar, Inc",0.20\nOak,-2.50\n')=={"Cedar, Inc":"0.30","Oak":"-2.50"}
assert totals("")=={}
assert totals("name,amount\nA,9999999999999999.99\nA,0.01\n")=={"A":"10000000000000000.00"}

print('ALT_ORACLE_COMPLETED_c8bb95e57ada4478899cdcd7219fecfc')
