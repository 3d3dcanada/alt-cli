import json,os,signal,subprocess,sys,time
child=subprocess.Popen([sys.executable,'-c','import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); time.sleep(30)'],stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
open(sys.argv[1],'w').write(str(child.pid))
time.sleep(.2)
for line in sys.stdin:
 req=json.loads(line)
 if 'id' not in req: continue
 if req['method']=='initialize': result={'protocolVersion':'2025-03-26','serverInfo':{'name':'bounded-audit','version':'1'},'capabilities':{'tools':{}}}
 elif req['method']=='tools/list': result={'tools':[]}
 else: result={}
 print(json.dumps({'jsonrpc':'2.0','id':req['id'],'result':result}),flush=True)
