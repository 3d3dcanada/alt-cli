import importlib.util, json, os, sys
from pathlib import Path
spec = importlib.util.spec_from_file_location('practice_greeting', Path.cwd() / 'greeting.py')
cases = []
try:
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    for name, value, expected in [('ordinary name', 'Cedar', 'Hello, Cedar!'), ('trim spaces', '  Cedar  ', 'Hello, Cedar!'), ('empty name', '', 'Hello, friend!'), ('spaces only', '   ', 'Hello, friend!')]:
        try:
            actual = module.greet(value)
            passed = actual == expected
            print(f'{name}: expected {expected!r}, got {actual!r}')
        except Exception as error:
            passed = False
            print(f'{name}: {error}')
        cases.append({'name': name, 'status': 'passed' if passed else 'failed'})
except Exception as error:
    print(f'Cannot load greeting.py: {error}')
    cases.append({'name': 'load greeting.py', 'status': 'failed'})
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema': 1, 'complete': True, 'tests': cases}))
sys.exit(0 if all(c['status'] == 'passed' for c in cases) else 1)
