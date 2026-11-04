from pathlib import Path
import os
import shutil
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parents[1]

with tempfile.TemporaryDirectory(prefix="ankipure-cli-") as folder:
    base=Path(folder)
    project=base/"project"
    project.mkdir()
    shutil.copy(ROOT/"ankipure",project/"ankipure")
    shutil.copytree(ROOT/"scripts",project/"scripts")
    (project/"manifest.json").write_text('{}')
    (project/"ankimarkable-pure").write_text('#!/bin/sh\nexit 0\n')
    (project/"ankimarkable-pure").chmod(0o755)
    commands=base/"commands"
    commands.mkdir()
    log=base/"ssh.log"
    request=base/"request.json"
    (commands/"ssh").write_text('''#!/usr/bin/env python3
import os,sys,json
from pathlib import Path
args=sys.argv[1:]
with open(os.environ['CLI_LOG'],'a') as log: log.write(json.dumps(args)+'\\n')
if '-M' in args or '-O' in args: sys.exit(0)
command=args[-1]
if 'hostname' in command: print(os.environ.get('CLI_IDENTITY','imx93-tatsu|aarch64|3.28.0.172')); sys.exit(0)
if command.startswith('tar '): sys.stdout.buffer.write(b'private backup'); sys.exit(0)
if 'cat >' in command and command.endswith('launcher-command.json'):
 data=sys.stdin.buffer.read();Path(os.environ['CLI_REQUEST']).write_bytes(data)
 with open(os.environ['CLI_REQUEST']+'.history','ab') as log: log.write(data)
 sys.exit(0)
if 'AnkiPureCLI:' in command:
 d=json.loads(Path(os.environ['CLI_REQUEST']).read_text()); print('AnkiPureCLI:'+d['nonce']+':ok:Anki opened'); sys.exit(0)
if command=='sh': sys.stdin.buffer.read()
if 'cat >' in command: sys.stdin.buffer.read()
''')
    (commands/"ssh").chmod(0o755)
    env=os.environ.copy()
    env.pop('SSH_CONTROL_PATH',None)
    env.update(PATH=f"{commands}:{env['PATH']}",CLI_LOG=str(log),CLI_REQUEST=str(request))

    def run(*args,environment=None):
        return subprocess.run(['bash',str(project/'ankipure'),*args],env=environment or env,capture_output=True,text=True)

    assert run('help').returncode==0
    assert not log.exists(), 'Help must not connect to a device'
    assert run('nonsense').returncode!=0
    package=base/'deck $(touch unexpected).apkg'
    package.write_bytes(b'local package')
    imported=run('import',str(package))
    assert imported.returncode==0, imported.stderr
    calls=[__import__('json').loads(line) for line in log.read_text().splitlines()]
    assert sum('-M' in c for c in calls)==1, 'One authentication for the whole operation'
    backup_call=next(i for i,c in enumerate(calls) if c[-1].startswith('tar '))
    upload_call=next(i for i,c in enumerate(calls) if 'cat >' in c[-1] and 'import.apkg' in c[-1])
    assert backup_call<upload_call, 'Backup must precede import'
    prepare_call=next(i for i,c in enumerate(calls) if 'ankimarkable-pure-tools prepare' in c[-1])
    assert upload_call<prepare_call, 'Prepare the imported content before browsing'
    assert 'AM_MATHPNG=/home/root/xovi/exthome/appload/ankimarkable-pure/mathpng' in calls[prepare_call][-1]
    assert 'AM_MATHPNG_RES=' in calls[prepare_call][-1]
    assert 'AM_MATH_CACHE=' in calls[prepare_call][-1]
    assert '--exclude=.ankimarkable-pure/ankiweb.txt' in calls[backup_call][-1]
    assert '--exclude=.ankimarkable-pure/sync-auth.json' in calls[backup_call][-1]
    assert all(str(package) not in ' '.join(c) for c in calls), 'Local names must never become remote shell code'
    assert not (base/'unexpected').exists()
    backups=list((project/'.backups').glob('*.tar.gz'))
    assert len(backups)==1 and backups[0].stat().st_mode & 0o777==0o600
    assert (project/'.backups').stat().st_mode & 0o777==0o700
    log.unlink()
    bad=env.copy();bad['CLI_IDENTITY']='other|aarch64|3.28.0.172'
    assert run('import',str(package),environment=bad).returncode!=0
    assert not any('cat >' in line or 'kill -TERM' in line for line in log.read_text().splitlines()), 'Unsupported devices must remain untouched'
    log.unlink()
    assert run('open').returncode==0
    assert __import__('json').loads(request.read_text())['action']=='open'
    assert any("printf '{}" in line and 'launcher-command.json' in line for line in log.read_text().splitlines()), 'Acknowledged requests must be cleared'
    (project/'scripts/install.sh').write_text('#!/bin/bash\nexit 0\n')
    (project/'scripts/install.sh').chmod(0o755)
    assert run('setup').returncode==0
    history=[__import__('json').loads(line) for line in Path(str(request)+'.history').read_text().splitlines()]
    assert history[-2]['action']=='reload' and history[-1]['action']=='open'
    assert history[-2]['nonce']!=history[-1]['nonce'], 'Sequential launcher requests must not be deduplicated'

print('CLI contracts passed: one authentication, backup-before-import, private backups, safe paths, model guard, and launcher request.')
