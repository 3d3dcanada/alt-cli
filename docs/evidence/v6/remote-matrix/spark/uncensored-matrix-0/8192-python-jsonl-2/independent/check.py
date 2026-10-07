import sys,os
sys.path.insert(0,os.getcwd())
from solution import records
assert records(' {"a":1}\n\n{"b":2}\n')==[{"a":1},{"b":2}]
for value in ["bad","[]"]:
 try:records("\n"+value)
 except ValueError as e:assert "2" in str(e)
 else:raise AssertionError()

print('ALT_ORACLE_COMPLETED_1854f6dbffc24deebd4818d21673b21f')
