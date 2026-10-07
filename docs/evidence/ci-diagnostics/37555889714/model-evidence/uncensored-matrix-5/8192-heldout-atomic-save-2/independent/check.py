import sys,os
sys.path.insert(0,os.getcwd())
from solution import save
from pathlib import Path
from unittest.mock import patch
import tempfile
with tempfile.TemporaryDirectory() as d:
 p=Path(d)/"x";p.write_text("old")
 with patch("os.replace",side_effect=OSError("fault")):
  try:save(p,"new")
  except OSError:pass
  else:raise AssertionError("did not replace")
 assert p.read_text()=="old" and len(list(Path(d).iterdir()))==1
 save(p,"cedar ☃");assert p.read_text()=="cedar ☃"

print('ALT_ORACLE_COMPLETED_55f946a165a14ecea9f6f02884e616ed')
