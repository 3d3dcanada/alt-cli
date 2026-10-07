import sys,os
sys.path.insert(0,os.getcwd())
import solution,os,tempfile
original=os.getcwd()
with tempfile.TemporaryDirectory() as d:
 try:os.chdir(d);assert solution.main()=="Hello, Cedar"
 finally:os.chdir(original)

print('ALT_ORACLE_COMPLETED_5b9f7f7430024b7b8f9ed46e400f0e53')
