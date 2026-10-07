import sys,os
sys.path.insert(0,os.getcwd())
from formatter import slug
assert slug(" Hello, CEDAR! ")=="hello-cedar"
assert slug("v2__ready")=="v2-ready"
assert slug("---")==""
assert slug("A  B")=="a-b"
assert slug("Café 99") == "caf-99"
import subprocess,sys
assert subprocess.check_output([sys.executable,"main.py"],text=True).strip()=="hello-cedar"

print('ALT_ORACLE_COMPLETED_543671c24c2c44a7a608123d1fe059fa')
