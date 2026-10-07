import sys,os
sys.path.insert(0,os.getcwd())
from solution import transfer
import sqlite3
c=sqlite3.connect(":memory:");c.execute("create table accounts(id integer primary key,balance integer)");c.executemany("insert into accounts values(?,?)",[(1,10),(2,0)]);c.commit()
transfer(c,1,2,4);assert c.execute("select balance from accounts order by id").fetchall()==[(6,),(4,)]
for args in [(1,3,2),(1,2,9),(1,1,1),(1,2,True),(1,2,-1)]:
 try:transfer(c,*args)
 except ValueError:pass
 else:raise AssertionError(args)
 assert c.execute("select balance from accounts order by id").fetchall()==[(6,),(4,)]

print('ALT_ORACLE_COMPLETED_47d000a9fff24670b68af92b24437fd1')
