import sys,os
sys.path.insert(0,os.getcwd())
from solution import Cache
now=[0];c=Cache(5,lambda:now[0]);c.set("a",None);assert c.get("a",7) is None
now[0]=4;c.set("a",2);now[0]=5;assert c.get("a")==2
now[0]=9;assert c.get("a","expired")=="expired"
z=Cache(0,lambda:0);z.set("x",1);assert z.get("x") is None

print('ALT_ORACLE_COMPLETED_e233811b48724e7f9dff9e7c19aba39d')
