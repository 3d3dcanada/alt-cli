import sys,os
sys.path.insert(0,os.getcwd())
from formatter import slug
assert slug(" Hello, CEDAR! ")=="hello-cedar"
assert slug("v2__ready")=="v2-ready"
assert slug("---")==""
assert slug("A  B")=="a-b"
