import sys,os
sys.path.insert(0,os.getcwd())
import solution,os,tempfile
original=os.getcwd()
with tempfile.TemporaryDirectory() as d:
 try:os.chdir(d);assert solution.main()=="Hello, Cedar"
 finally:os.chdir(original)

print('ALT_ORACLE_COMPLETED_5e50e092dbcc4c88908ad433642ac694')
