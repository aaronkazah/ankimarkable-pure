"""Check resource-map refresh on supported firmware upgrades."""
from pathlib import Path
import os
import socket
import subprocess
import tempfile

assert Path("/opt/codex/sysroots").is_dir(), "Run in the SDK container"
source = Path("/project/scripts/start-launcher.sh").read_text()
with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)
    scripts = root / "scripts"
    scripts.mkdir()
    commands = root / "commands"
    commands.mkdir()
    firmware = root / "os-release"
    runtime = root / "runtime"
    resource_map = runtime / "exthome/qt-resource-rebuilder"
    resource_map.mkdir(parents=True)
    (runtime / "start").write_text("#!/bin/sh\nexit 0\n")
    (runtime / "start").chmod(0o755)
    log = root / "rebuilds"
    (commands / "hostname").write_text("#!/bin/sh\necho imx93-tatsu\n")
    (commands / "ssh").write_text(
        '#!/bin/bash\nwhile [[ $# -gt 0 ]]; do '
        'case "$1" in -o|-S) shift 2 ;; *) shift; break ;; esac; '
        'done\nexec /bin/sh -c "$*"\n'
    )
    for command in commands.iterdir():
        command.chmod(0o755)
    script = scripts / "start-launcher.sh"
    script.write_text(source.replace("/etc/os-release", str(firmware))
                      .replace("/home/root/xovi", str(runtime)))
    (scripts / "tablet-launcher-setup.sh").write_text(
        f"#!/bin/sh\necho rebuild >> {log}\n"
        f"echo ready > {resource_map}/hashtab\n"
        f"sed -n 's/^IMG_VERSION=\"\\(.*\\)\"/\\1/p' "
        f"{firmware} > {resource_map}/hashtab-firmware\n"
    )
    environment = os.environ | {"PATH": f"{commands}:/usr/bin:/bin"}
    environment.pop("SSH_CONTROL_PATH", None)
    with socket.socket(socket.AF_UNIX) as ready:
        ready.bind("/tmp/qtfb.sock")
        for version in ("3.28.0.172", "3.29.1.0"):
            firmware.write_text(f'IMG_VERSION="{version}"\n')
            result = subprocess.run(["bash", str(script)], env=environment,
                                    capture_output=True, text=True)
            assert result.returncode == 0, result.stderr
            assert (resource_map / "hashtab-firmware").read_text().strip() == version
        assert log.read_text().splitlines() == ["rebuild", "rebuild"]
        assert subprocess.run(["bash", str(script)], env=environment,
                              capture_output=True).returncode == 0
        assert len(log.read_text().splitlines()) == 2
        firmware.write_text('IMG_VERSION="3.99.0.0"\n')
        assert subprocess.run(["bash", str(script)], env=environment,
                              capture_output=True).returncode != 0
        assert len(log.read_text().splitlines()) == 2
    Path("/tmp/qtfb.sock").unlink()
print("Launcher contracts passed: upgrade rebuilds, reuse, unsupported refusal")
