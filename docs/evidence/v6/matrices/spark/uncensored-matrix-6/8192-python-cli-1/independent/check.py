import sys,os
sys.path.insert(0,os.getcwd())
import subprocess,sys
def run(*args):return subprocess.run([sys.executable,"solution.py",*args],capture_output=True,text=True)
r=run("--name","Cedar Tree","--times","2");assert r.returncode==0 and r.stdout=="Hello, Cedar Tree!\n"*2
assert run().returncode!=0
assert run("--name","X","--times","0").returncode!=0

print('ALT_ORACLE_COMPLETED_763548f47daa4a6abb6178820bd60f4b')
