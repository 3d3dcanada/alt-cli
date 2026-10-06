"""Deterministic MCP boundary fixture, never an inference model."""
import json, sys
from http.server import HTTPServer, BaseHTTPRequestHandler
calls = 0

def handle(m):
    global calls
    if 'id' not in m: return None
    method = m['method']
    if method == 'initialize': result = {'protocolVersion':'2025-03-26','capabilities':{'tools':{}},'serverInfo':{'name':'fixture','version':'1'}}
    elif method == 'tools/list': result = {'tools':[{'name':'counter','description':'fixture','inputSchema':{'type':'object','properties':{}}},{'name':'failure','description':'fixture failure','inputSchema':{'type':'object'}}]}
    elif method == 'tools/call':
        calls += 1
        if m['params']['arguments'].get('disconnect'): raise EOFError('injected disconnect')
        result = {'content':[{'type':'text','text':str(calls)}], 'isError':m['params']['name']=='failure'}
    else: return {'jsonrpc':'2.0','id':m['id'],'error':{'code':-32601,'message':'unknown'}}
    return {'jsonrpc':'2.0','id':m['id'],'result':result}

if '--http' in sys.argv:
    class H(BaseHTTPRequestHandler):
        def log_message(self,*args): pass
        def do_POST(self):
            if self.headers.get('Authorization') != 'Bearer fixture-secret': self.send_error(401); return
            m=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            if m['method'] != 'initialize' and (self.headers.get('Mcp-Session-Id') != 'session-fixture' or self.headers.get('MCP-Protocol-Version') != '2025-03-26'):
                self.send_error(400); return
            try: value=handle(m)
            except EOFError: self.close_connection=True; return
            body = ('data: '+json.dumps(value)+'\n\n').encode() if value else b''
            if value and '--sse-variants' in sys.argv:
                # Valid SSE: BOM, comments, unrelated event, CRLF, multi-line data,
                # and data fields without the optional space after the colon.
                body = ('\ufeff:keepalive\r\n\r\ndata:{"jsonrpc":"2.0","id":999,"result":{}}\r\n\r\n'
                        + '\r\n'.join('data:' + line for line in json.dumps(value, indent=2).splitlines())
                        + '\r\n\r\n').encode()
            self.send_response(200 if value else 202)
            self.send_header('Content-Type','text/event-stream')
            self.send_header('Mcp-Session-Id','session-fixture')
            self.send_header('Content-Length',str(len(body))); self.end_headers(); self.wfile.write(body)
    server=HTTPServer(('127.0.0.1',0),H)
    print(server.server_port,flush=True); server.serve_forever()
else:
    for line in sys.stdin:
        try: result=handle(json.loads(line))
        except EOFError: sys.exit(1)
        if result: print(json.dumps(result),flush=True)
