"""Check language tools before spending a live model's allowance."""
import os
import shutil
import subprocess
import sys


def inspect(cases, env=None):
    env = dict(os.environ if env is None else env)
    files = [path for case in cases for path in case['files']]
    commands = [[sys.executable, '--version']]
    if any(path.endswith('.rs') for path in files):
        commands += [['cargo', '--version'], ['rustc', '--version']]
    if any(path.endswith(('.js', '.mjs')) for path in files):
        commands += [['node', '--version']]
    checks = []
    for command in commands:
        executable = shutil.which(command[0], path=env.get('PATH', ''))
        entry = dict(command=command, executable=executable, passed=False)
        try:
            if executable is None:
                raise FileNotFoundError('Required language tool is not on PATH')
            result = subprocess.run([executable] + command[1:], env=env,
                                    capture_output=True, text=True, timeout=30)
            entry.update(exit=result.returncode, stdout=result.stdout[-4000:],
                         stderr=result.stderr[-4000:], passed=result.returncode == 0)
        except (OSError, subprocess.TimeoutExpired) as error:
            entry['error'] = str(error)
        checks.append(entry)
    return dict(passed=all(c['passed'] for c in checks), checks=checks,
                scope='Language tool startup only; this does not verify source behavior or model quality')
