"""Verify compile-time provenance and construct a resolved CycloneDX SBOM."""
import hashlib,json,subprocess,tomllib
from pathlib import Path

def source_inputs(repo):
    paths=[]
    for name in ['Cargo.toml','Cargo.lock','rust-toolchain.toml','build.rs','src','assets','prompts','skills','.cargo/config.toml']:
        path=repo/name
        paths.extend(path.rglob('*') if path.is_dir() else [path])
    return {p.relative_to(repo).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths) if p.is_file()}
def verify(repo,binary):
    build=json.loads(subprocess.check_output([str(binary),'build-info'],text=True))
    inputs=source_inputs(repo)
    if inputs!=build.get('inputs'):
        changed=sorted(k for k in set(inputs)|set(build.get('inputs',{})) if inputs.get(k)!=build.get('inputs',{}).get(k))
        raise ValueError('Binary source provenance does not match checkout; rebuild before packaging: '+', '.join(changed[:12]))
    h=hashlib.sha256()
    for name,digest in sorted(inputs.items()):
        encoded=name.encode();h.update(len(encoded).to_bytes(8,'little'));h.update(encoded);h.update(digest.encode())
    assert build['source_sha256']==h.hexdigest(),'Invalid embedded source fingerprint'
    assert build['target']=='x86_64-unknown-linux-gnu','Unexpected binary target'
    assert build['rustc']==subprocess.check_output(['rustc','--version'],text=True).strip(),'Packaging toolchain differs from compile-time toolchain; use the matching toolchain for notices'
    return build

def sbom(repo,metadata,build,binary_sha):
    root=metadata['resolve']['root'];nodes={n['id']:n for n in metadata['resolve']['nodes']};packages={p['id']:p for p in metadata['packages']};roles={};edges={};todo=[(root,'runtime')]
    while todo:
        node,role=todo.pop()
        if role in roles.get(node,set()):continue
        roles.setdefault(node,set()).add(role);edges.setdefault(node,set())
        for dependency in nodes[node]['deps']:
            kinds={kind['kind'] or 'normal' for kind in dependency['dep_kinds']}
            allowed=kinds-{'dev'}
            if not allowed:continue
            edges[node].add(dependency['pkg'])
            for kind in allowed:todo.append((dependency['pkg'],'build' if kind=='build' or role=='build' else 'runtime'))
    checksums={(p['name'],p['version']):p.get('checksum') for p in tomllib.loads((repo/'Cargo.lock').read_text())['package']}
    def component(id):
        item=packages[id];ref='pkg:cargo/'+item['name']+'@'+item['version']
        c={'type':'application' if id==root else 'library','bom-ref':ref,'name':item['name'],'version':item['version'],'purl':ref,'scope':'required','properties':[{'name':'alt:dependency-roles','value':','.join(sorted(roles[id]))}]}
        if item.get('license'):c['licenses']=[{'expression':item['license']}]
        checksum=binary_sha if id==root else checksums.get((item['name'],item['version']))
        if checksum:c['hashes']=[{'alg':'SHA-256','content':checksum}]
        if item.get('repository'):c['externalReferences']=[{'type':'vcs','url':item['repository']}]
        return c
    refs={id:component(id)['bom-ref'] for id in roles}
    return {'bomFormat':'CycloneDX','specVersion':'1.6','version':1,'metadata':{'component':component(root),'properties':[{'name':'alt:source-sha256','value':build['source_sha256']},{'name':'alt:rustc','value':build['rustc']},{'name':'alt:target','value':build['target']},{'name':'alt:scope','value':'Resolved normal and build dependencies for target; development-only edges excluded. Runtime binaries and model weights are acquired separately.'}]},'components':[component(id) for id in sorted(roles) if id!=root],'dependencies':[{'ref':refs[id],'dependsOn':sorted(refs[dep] for dep in edges[id])} for id in sorted(roles)]}
