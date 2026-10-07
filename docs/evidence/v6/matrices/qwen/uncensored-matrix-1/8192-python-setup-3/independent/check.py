import sys,os
sys.path.insert(0,os.getcwd())
import solution,os,tempfile
original=os.getcwd()
with tempfile.TemporaryDirectory() as d:
 try:os.chdir(d);assert solution.main()=="Hello, Cedar"
 finally:os.chdir(original)

print('ALT_ORACLE_COMPLETED_65065e054db44d429fa74ac6aca89a5e')
