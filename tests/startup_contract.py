"""Exercise the boot guard in the disposable SDK container."""
from pathlib import Path
import os
import socket
import subprocess
import tempfile

assert Path('/opt/codex/sysroots').is_dir(), 'Run only in the disposable SDK container'
script = Path('/project/scripts/boot-launcher.sh')
with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)
    runtime = Path('/home/root/xovi')
    runtime.mkdir(parents=True, exist_ok=True)
    resource_map = runtime / 'exthome/qt-resource-rebuilder/hashtab'
    resource_map.parent.mkdir(parents=True, exist_ok=True)
    resource_map.write_text('test resource map')
    log = root / 'calls'
    for name in ['start', 'stock']:
        target = runtime / name
        target.write_text(f'#!/bin/sh\necho {name} >> "$CALL_LOG"\n')
        target.chmod(0o755)
    commands = {
        'hostname': '#!/bin/sh\nprintf "%s\\n" "$TEST_HOST"\n',
        'grep': '#!/bin/sh\nif test "$TEST_FIRMWARE" = approved; then exit 0; else exit 1; fi\n',
        'seq': '#!/bin/sh\necho 1\n',
        'sleep': '#!/bin/sh\nexit 0\n',
    }
    for name, body in commands.items():
        target = root / name; target.write_text(body); target.chmod(0o755)
    env = os.environ | {'PATH': f'{root}:/usr/bin:/bin', 'CALL_LOG': str(log),
                        'TEST_HOST': 'imx93-tatsu', 'TEST_FIRMWARE': 'approved'}
    def run(**changes):
        log.unlink(missing_ok=True)
        result = subprocess.run(['/bin/sh', str(script)], env=env | changes, capture_output=True)
        return result.returncode, log.read_text().splitlines() if log.exists() else []
    assert run(TEST_HOST='other-device') == (0, [])
    assert run(TEST_FIRMWARE='updated') == (0, [])
    assert run() == (1, ['start', 'stock'])
    with socket.socket(socket.AF_UNIX) as ready:
        ready.bind('/tmp/qtfb.sock')
        assert run() == (0, [])
    Path('/tmp/qtfb.sock').unlink()
print('Passed: wrong device, changed firmware, rollback on failed startup, already running')
