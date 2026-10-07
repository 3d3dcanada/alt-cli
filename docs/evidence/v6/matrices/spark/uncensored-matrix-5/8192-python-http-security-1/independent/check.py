import sys,os
sys.path.insert(0,os.getcwd())
import socket,subprocess,sys,time,json,urllib.request,urllib.parse
s=socket.socket();s.bind(('127.0.0.1',0));port=s.getsockname()[1];s.close()
p=subprocess.Popen([sys.executable,'server.py','--port',str(port)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
try:
 for _ in range(100):
  try:urllib.request.urlopen(f'http://127.0.0.1:{port}/users?name=Alice',timeout=.2);break
  except Exception:time.sleep(.03)
 def get(path):
  with urllib.request.urlopen(f'http://127.0.0.1:{port}'+path,timeout=2) as r:return r.status,json.load(r)
 assert get('/health')==(200,{'status':'ok'})
 assert get('/users?name=Alice')==(200,['Alice'])
 assert get('/users?name='+urllib.parse.quote("' OR 1=1 --"))==(200,[])
 assert get('/users?name=Absent')==(200,[])
 assert get('/users?name='+urllib.parse.quote("O'Brien"))==(200,[])
 assert get('/users?name='+urllib.parse.quote("Alice' UNION SELECT 'injected' --"))==(200,[])
finally:
 p.terminate();p.wait(timeout=3)

print('ALT_ORACLE_COMPLETED_03a4f7d6b61e41d39a9046ef965a9887')
