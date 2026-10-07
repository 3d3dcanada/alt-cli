import datetime, hashlib, json, os, pathlib, subprocess, tarfile, tempfile

root = pathlib.Path('/workspace/.alt-evaluations/v6/published-beta')
root.mkdir(parents=True, exist_ok=True)
gh = '/workspace/.alt-tools/gh'
repo = '3d3dcanada/alt-cli'
tag = 'v0.6.0-beta.1'
commit = '7ea6533d2acafe947d6ffdcc7e68809d94305005'
name = 'alt-0.6.0-linux-x86_64.tar.gz'

def require(condition, message):
    if not condition:
        raise ValueError(message)

def command(args, stem, **kwargs):
    result = subprocess.run(args, capture_output=True, text=True, **kwargs)
    (root / (stem + '.stdout')).write_text(result.stdout)
    (root / (stem + '.stderr')).write_text(result.stderr)
    require(result.returncode == 0, f'{stem} failed, exit {result.returncode}; inspect its saved log')
    return result.stdout

release = json.loads(command([gh, 'api', f'repos/{repo}/releases/tags/{tag}'], 'release-metadata'))
require(release['prerelease'] and not release['draft'] and release['tag_name'] == tag, 'Release status differs')
expected = {name, name + '.sha256', name + '.sigstore.jsonl', 'release-gate.json', 'dependency-audit.json', 'attestation-verification.json'}
require({v['name'] for v in release['assets']} == expected, 'Release asset set differs')
assets = root / 'assets'
assets.mkdir(exist_ok=True)
if not all((assets / filename).is_file() for filename in expected):
    command([gh, 'release', 'download', tag, '--repo', repo, '--dir', str(assets)], 'download')
digests = {}
for asset in release['assets']:
    path = assets / asset['name']
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    require(path.stat().st_size == asset['size'], 'Asset size differs')
    require(asset['digest'] == 'sha256:' + digest, 'GitHub asset digest differs')
    digests[asset['name']] = digest
require((assets / (name + '.sha256')).read_text().split() == [digests[name], name], 'Checksum differs')
command([gh, 'attestation', 'verify', str(assets / name), '--repo', repo,
         '--bundle', str(assets / (name + '.sigstore.jsonl')),
         '--custom-trusted-root', str(root / 'verified-trusted-root.jsonl'),
         '--signer-workflow', repo + '/.github/workflows/publish.yml',
         '--source-ref', 'refs/tags/' + tag, '--source-digest', commit,
         '--format', 'json'], 'independent-attestation')
gate = json.loads((assets / 'release-gate.json').read_text())
require(gate['archive_sha256'] == digests[name], 'Release gate archive differs')
require(gate['passed'] and gate['upgrade_gate'] == 'measured', 'Exact package or upgrade gate failed')
require(len(gate['checks']) == 7 and all(r['passed'] for r in gate['checks']), 'Incomplete package gate')
audit = json.loads((assets / 'dependency-audit.json').read_text())
require(audit['vulnerabilities']['count'] == 0 and not audit['warnings'], 'Dependency audit failed')
with tempfile.TemporaryDirectory(prefix='alt-published-consumer-') as folder:
    temp = pathlib.Path(folder)
    with tarfile.open(assets / name) as archive:
        archive.extractall(temp, filter='data')
    package = temp / name.removesuffix('.tar.gz')
    build = json.loads((package / 'BUILD.json').read_text())
    require(build['commit'] == commit and build['dirty'] is False, 'Package is not a clean tagged build')
    require(build['source_sha256'] == '6a040feeaa4fd882d0db31e052c218be3cf1a757f0a6236b836183e64dae2ecd', 'Compiled inputs differ')
    env = os.environ.copy()
    env['ALT_PREFIX'] = str(temp / 'prefix')
    command(['bash', str(package / 'install.sh'), '--data-dir', str(temp / 'state')], 'consumer-install', env=env)
    binary = temp / 'prefix/bin/alt'
    version = command([str(binary), '--version'], 'consumer-version').strip()
    require(version == 'alt 0.6.0', 'Installed version differs')
    installed_build = json.loads(command([str(binary), 'build-info'], 'consumer-build'))
    require(installed_build == build, 'Installed build differs')
    binary_digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    require(binary_digest == json.loads((package / 'PLATFORM.json').read_text())['sha256'], 'Installed bytes differ')
report = {'schema': 1, 'checked_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
          'release_url': release['html_url'], 'tag': tag, 'commit': commit,
          'assets_sha256': digests, 'github_asset_digests_verified': True,
          'attestation_verified': True, 'source_commit_enforced': True,
          'package_checks_passed': len(gate['checks']), 'previous_version_recovery': True,
          'dependency_advisories': 0, 'consumer_install_passed': True,
          'installed_binary_sha256': binary_digest, 'build': build,
          'trust_bootstrap': json.loads((root / 'trust-bootstrap.json').read_text()),
          'scope': 'Downloaded the actual public release assets, checked GitHub digests/checksum and identity-signed provenance including exact source commit, inspected attached exact-package gates and audit, then installed into a disposable local prefix. Official signed TUF metadata was obtained via GitHub because cloud CDN access returned HTTP 403; its signatures and expiry were verified using the trust anchor embedded in the official GitHub CLI. No physical GPU or new model inference.'}
(root / 'verification.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({k: report[k] for k in ['release_url', 'attestation_verified', 'package_checks_passed', 'consumer_install_passed', 'installed_binary_sha256']}))
