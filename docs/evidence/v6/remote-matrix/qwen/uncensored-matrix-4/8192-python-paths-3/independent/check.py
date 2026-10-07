import sys,os
sys.path.insert(0,os.getcwd())
from solution import resolve_file
from pathlib import Path
import tempfile
with tempfile.TemporaryDirectory() as d:
 base=Path(d);root=base/"root";root.mkdir();(root/"sub").mkdir();(root/"sub/a").write_text("x");(root/"link").symlink_to(base)
 assert resolve_file(root,"sub/a")==root/"sub/a"
 for path in ["../x",str(base),".","link/x"]:
  try:resolve_file(root,path)
  except ValueError:pass
  else:raise AssertionError(path)

print('ALT_ORACLE_COMPLETED_7bce427d89f14896b1a807ef1ce982a6')
