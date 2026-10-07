#!/usr/bin/env python3
"""Execute skill helpers against broken/fixed fixtures. No inference or quality claim."""
import argparse,hashlib,json,os,shutil,subprocess,tempfile,threading
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--alt',type=Path,default=Path('target/debug/alt'));p.add_argument('--browser',action='store_true');p.add_argument('--chromium',default='/usr/bin/chromium');p.add_argument('--output',type=Path);a=p.parse_args();binary=a.alt.resolve();rows=[]
    with tempfile.TemporaryDirectory(prefix='alt-skill-checks-') as directory:
        root=Path(directory)
        def helper(project,skill,name='build',inputs=None):
            result=subprocess.run([str(binary),'--data-dir',str(root/'state'),'--access','trusted','skills','run',skill,name,'--input',json.dumps(inputs or {})],cwd=project,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'},capture_output=True,text=True,timeout=180)
            assert result.stdout.strip(),result.stderr
            return result.returncode,json.loads(result.stdout)
        for language,skill in [('python','python-repair'),('rust','rust-diagnostics'),('javascript','javascript-setup'),('configuration','configuration-data')]:
            project=root/language;project.mkdir()
            if language=='configuration':
                source=project/'solution.py';broken='def validate(data):return data\n'
                fixed="def validate(data):\n port=data.get('port',8080);host=data.get('host','localhost')\n if type(port) is not int or not 1<=port<=65535 or not isinstance(host,str) or not host:raise ValueError('invalid config')\n return {'host':host,'port':port}\n"
                (project/'test_solution.py').write_text("import unittest\nfrom solution import validate\nclass Behavior(unittest.TestCase):\n def test_defaults(self):self.assertEqual(validate({}),{'host':'localhost','port':8080})\n def test_types_and_bounds(self):\n  for value in (True,0,65536,'80'):\n   with self.subTest(port=value),self.assertRaises(ValueError):validate({'port':value})\n def test_empty_host(self):\n  with self.assertRaises(ValueError):validate({'host':''})\n")
            elif language=='python':
                source=project/'solution.py';broken='def add(a,b):return a-b\n';fixed='def add(a,b):return a+b\n'
                (project/'test_solution.py').write_text('import unittest\nfrom solution import add\nclass Behavior(unittest.TestCase):\n def test_sum(self):self.assertEqual(add(4,3),7)\n def test_zero(self):self.assertEqual(add(0,5),5)\n')
            elif language=='rust':
                (project/'Cargo.toml').write_text('[package]\nname="skill_fixture"\nversion="0.1.0"\nedition="2021"\n');(project/'src').mkdir()
                source=project/'src/lib.rs';tests='\n#[cfg(test)] mod tests {#[test] fn sum(){assert_eq!(super::add(4,3),7);}}\n'
                broken='pub fn add(a:i32,b:i32)->i32{a-b}'+tests;fixed='pub fn add(a:i32,b:i32)->i32{a+b}'+tests
                source.write_text(broken)
                subprocess.run(['cargo','generate-lockfile','--offline'],cwd=project,check=True,capture_output=True)
            else:
                (project/'package.json').write_text(json.dumps({'name':'skill-fixture','version':'1.0.0','scripts':{'test':'node --test --test-reporter=tap test.cjs'}}))
                source=project/'solution.cjs';broken='exports.add=(a,b)=>a-b;\n';fixed='exports.add=(a,b)=>a+b;\n'
                (project/'test.cjs').write_text("const test=require('node:test'),assert=require('node:assert/strict'),{add}=require('./solution.cjs');test('sum',()=>assert.equal(add(4,3),7));\n")
            source.write_text(broken);code,before=helper(project,skill);assert code!=0 and before['status']=='failed',before
            source.write_text(fixed);code,after=helper(project,skill);assert code==0 and after['status']=='passed' and after['evidence']['tests_run']>0,after
            rows.append({'skill':skill,'broken_rejected':True,'fixed_passed':True,'actual_helper':after})
        if a.browser:
            class Page(BaseHTTPRequestHandler):
                fixed=False
                def log_message(self,*_):pass
                def do_GET(self):
                    content=("<button id='go' onclick=\"document.querySelector('#result').textContent='Ready'\">Go</button><p id='result'>Pending</p>" if self.fixed else "<button id='go'>Go</button><p id='result'>Pending</p>").encode();self.send_response(200);self.send_header('Content-Length',str(len(content)));self.end_headers();self.wfile.write(content)
            server=ThreadingHTTPServer(('127.0.0.1',0),Page);threading.Thread(target=server.serve_forever,daemon=True).start();project=root/'browser';project.mkdir()
            try:
                inputs={'url':f'http://127.0.0.1:{server.server_port}/','click':'#go','selector':'#result','contains':'Ready','executable':a.chromium}
                code,before=helper(project,'browser-checks','browser',inputs);assert code!=0 and before['status']=='findings',before
                Page.fixed=True;code,after=helper(project,'browser-checks','browser',inputs);assert code==0 and after['status']=='passed',after
                assert Path(after['input']['screenshot']).is_file()
                if a.output:
                    a.output.parent.mkdir(parents=True,exist_ok=True);image=a.output.parent/'skills-browser-checks.png';shutil.copy2(after['input']['screenshot'],image)
                    after['retained_screenshot']=image.name;after['screenshot_sha256']=hashlib.sha256(image.read_bytes()).hexdigest()
                rows.append({'skill':'browser-checks','broken_rejected':True,'fixed_passed':True,'actual_helper':after})
            finally:server.shutdown();server.server_close()
        report={'scope':'Executed tool recipes with real behavior assertions; no LLM, no skill-versus-baseline quality comparison','browser_executed':a.browser,'rows':rows}
        if a.output:a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n')
    print('PASS: '+', '.join(r['skill'] for r in rows)+' detect broken behavior and accept the unchanged checks after repair.')

if __name__=='__main__':main()
