import sys,os
sys.path.insert(0,os.getcwd())
from solution import migrate
import sqlite3
c=sqlite3.connect(":memory:");c.execute("create table items(id integer primary key,name text not null)");c.execute("insert into items values(7,'Cedar')");c.commit()
migrate(c);migrate(c);assert c.execute("select * from items").fetchall()==[(7,"Cedar",1)]
d=sqlite3.connect(":memory:");migrate(d);d.execute("insert into items(name) values('New')");assert d.execute("select active from items").fetchone()==(1,)

print('ALT_ORACLE_COMPLETED_99cf195362704e17a93774310cfed670')
