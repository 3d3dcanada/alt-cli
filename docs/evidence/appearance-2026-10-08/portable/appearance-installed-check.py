#!/usr/bin/env python3
"""Exercise saved appearance through the installed optimized package's real PTY."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import sys
import tarfile
import tempfile

repo = Path('/workspace/alt-cli')
evidence = Path('/workspace/.alt-style-finalization/portable')
archive = evidence / 'alt-0.6.0-linux-x86_64.tar.gz'
expected = json.loads((evidence / 'build-receipt.json').read_text())
with tempfile.TemporaryDirectory(prefix='installed-appearance-', dir=evidence.parent) as temporary:
    root = Path(temporary)
    with tarfile.open(archive) as handle:
        handle.extractall(root / 'package', filter='data')
    package = next((root / 'package').iterdir())
    prefix = root / 'installed'
    journey = root / 'journey'
    journey.mkdir()
    environment = {**os.environ, 'ALT_PREFIX': str(prefix), 'ALT_DATA_DIR': str(journey / 'state')}
    subprocess.run(['bash', str(package / 'install.sh')], env=environment, check=True)
    binary = prefix / 'bin/alt'
    binary_sha = hashlib.sha256(binary.read_bytes()).hexdigest()
    assert binary_sha == expected['binary_sha256']
    os.environ['ALT_TEST_BINARY'] = str(binary)
    os.environ['ALT_TUI_SCREENSHOTS'] = str(evidence / 'installed-appearance-screenshots')
    sys.path.insert(0, str(repo / 'scripts'))
    from smoke_appearance_tui import Terminal, apply_theme, apply_layout, contrast, navigation_mode, preferences, quit_cleanly

    term = Terminal(journey, 80, 24)
    try:
        term.wait_page('Home')
        apply_theme(term, 'Aurora', 'aurora')
        apply_layout(term, 'Focus', 'focus')
        selected = preferences(term)
        assert selected['theme'] == 'aurora' and selected['layout'] == 'focus'
        assert navigation_mode(term) == 'focus'
        before = contrast(term)
        term.capture('installed-aurora-focus')
        quit_cleanly(term)
    finally:
        term.stop()

    term = Terminal(journey, 80, 24)
    try:
        term.wait_page('Home')
        restored = preferences(term)
        assert restored['theme'] == 'aurora' and restored['layout'] == 'focus'
        assert navigation_mode(term) == 'focus'
        after = contrast(term)
        for field in ['shell_background', 'brand_foreground']:
            assert before[field] == after[field], field
        term.capture('installed-aurora-focus-restarted')
        quit_cleanly(term)
    finally:
        term.stop()

    assert hashlib.sha256(binary.read_bytes()).hexdigest() == binary_sha
    receipt = {
        'schema': 1,
        'passed': True,
        'scope': 'Installed optimized portable package; one real PTY appearance and restart journey. Full theme/layout matrix is covered separately.',
        'archive_sha256': hashlib.sha256(archive.read_bytes()).hexdigest(),
        'binary_sha256': binary_sha,
        'build_commit': expected['commit'],
        'source_sha256': expected['source_sha256'],
        'theme': 'aurora',
        'layout': 'focus',
        'theme_and_layout_saved_through_public_menus': True,
        'theme_and_layout_retained_after_restart': True,
        'rendered_colors_retained_after_restart': True,
        'terminal_restored_on_both_exits': True,
        'minimum_observed_text_contrast': min(before['minimum_text_contrast'], after['minimum_text_contrast']),
        'model_weights': False,
        'published': False,
    }
    (evidence / 'appearance-installed.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print('PASS: installed portable Aurora / Focus choices, saved state and visible colors after restart; terminal restored on both exits.')
