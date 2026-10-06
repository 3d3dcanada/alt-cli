import sys,os
sys.path.insert(0,os.getcwd())
from app import label
assert label("Cedar Project")=="cedar-project"
assert label("  A   B  ")=="a-b"
