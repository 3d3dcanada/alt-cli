import sys,os
sys.path.insert(0,os.getcwd())
from app import label
assert label("Cedar Project")=="cedar-project"
assert label("  A   B  ")=="a-b"
import subprocess,sys,importlib
assert subprocess.check_output([sys.executable,"app.py"],text=True).strip()=="cedar-project"
import vendor.slugify as bundled
from pathlib import Path
for module in list(sys.modules.values()):
    module_path=getattr(module,"__file__",None)
    if module_path and Path(module_path).resolve()==Path("vendor/slugify.py").resolve():
        module.slugify=lambda text:"ALT_BUNDLED_RECEIPT:"+text
sys.modules.pop("app",None)
assert importlib.import_module("app").label("probe")=="ALT_BUNDLED_RECEIPT:probe", "label must call the bundled helper, not duplicate its logic"

print('ALT_ORACLE_COMPLETED_659ac1fae958435c916a1069b72b620e')
