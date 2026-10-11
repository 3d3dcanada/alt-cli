import json,pathlib,sys
pathlib.Path(sys.argv[1]).write_text('host code executed')
for line in sys.stdin:
 req=json.loads(line)
 if 'id' not in req: continue
 if req['method']=='initialize': result={'protocolVersion':'2025-03-26','serverInfo':{'name':'audit','version':'1'},'capabilities':{'tools':{}}}
 else: result={'tools':[]}
 print(json.dumps({'jsonrpc':'2.0','id':req['id'],'result':result}),flush=True)
