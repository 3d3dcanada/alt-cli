import sys,os
sys.path.insert(0,os.getcwd())
import asyncio
from solution import gather_values
async def main():
 active=[0,0]
 def make(n):
  async def work():
   active[0]+=1;active[1]=max(active);await asyncio.sleep(.005*(4-n));active[0]-=1;return n
  return work
 assert await gather_values([make(i) for i in range(4)],2)==[0,1,2,3]
 assert active[1]==2
 assert await gather_values([],1)==[]
asyncio.run(main())

print('ALT_ORACLE_COMPLETED_94252c3d977a4d0fa63650887e5abd09')
