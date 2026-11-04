"""Exercise enabling and disabling automatic startup against a simulated Paper Pure."""
from pathlib import Path
import os
import subprocess
import tempfile

SOURCE = (Path(__file__).resolve().parents[1] / "scripts/startup.sh").read_text()


def run(action, *, host="imx93-tatsu", firmware="3.28.0.172", collapses=True, installed=False):
    with tempfile.TemporaryDirectory(prefix="pure-startup-contract-") as temporary:
        base = Path(temporary)
        etc = base / "etc"
        (etc / "systemd/system").mkdir(parents=True)
        (etc / "os-release").write_text(f'IMG_VERSION="{firmware}"\n')
        unit = etc / "systemd/system/ankimarkable-start.service"
        if installed:
            unit.write_text("[Unit]\n")
        home = base / "home"
        launcher = home / "xovi/exthome/appload/ankimarkable-pure/scripts/boot-launcher.sh"
        launcher.parent.mkdir(parents=True)
        launcher.write_text("#!/bin/sh\n")
        launcher.chmod(0o755)
        mounts = base / "mounts"
        mounts.write_text(f"/dev/root / ext4 ro,relatime 0 0\noverlay {etc} overlay rw 0 0\n")
        log = base / "calls"
        commands = base / "commands"
        commands.mkdir()
        stubs = {
            "ssh": '#!/bin/bash\nwhile [[ $# -gt 0 ]]; do case "$1" in -o|-S) shift 2 ;; *) shift; break ;; esac; done\nexec /bin/sh -c "$*"\n',
            "hostname": f"#!/bin/sh\necho {host}\n",
            "mount": f'#!/bin/sh\necho "mount $*" >> {log}\n'
                     f'case "$2" in remount,rw) sed -i "s# / ext4 ro# / ext4 rw#" {mounts} ;; remount,ro) sed -i "s# / ext4 rw# / ext4 ro#" {mounts} ;; esac\n',
            "umount": f'#!/bin/sh\nif test -e {unit}; then state=present; else state=absent; fi\necho "umount $* unit-$state" >> {log}\n'
                      f'test "${{KEEP_OVERLAY:-0}}" = 1 || sed -i "/ overlay /d" {mounts}\n',
            "systemctl": f'#!/bin/sh\necho "systemctl $*" >> {log}\n',
            "systemd-analyze": f'#!/bin/sh\necho "systemd-analyze $*" >> {log}\n',
            "systemd-run": f'#!/bin/sh\necho "systemd-run $*" >> {log}\n',
            "sync": "#!/bin/sh\nexit 0\n",
        }
        for name, body in stubs.items():
            (commands / name).write_text(body)
            (commands / name).chmod(0o755)
        script = base / "startup.sh"
        script.write_text(SOURCE.replace("/proc/mounts", str(mounts)).replace("/etc", str(etc)).replace("/home/root", str(home)))
        environment = os.environ | {"PATH": f"{commands}:{os.environ['PATH']}", "KEEP_OVERLAY": "0" if collapses else "1"}
        environment.pop("SSH_CONTROL_PATH", None)
        result = subprocess.run(["bash", str(script), "root@test-device", action], env=environment, capture_output=True, text=True)
        calls = log.read_text().splitlines() if log.exists() else []
        return result, calls, unit.read_text() if unit.exists() else None, mounts.read_text()


def index(calls, prefix):
    return next(i for i, call in enumerate(calls) if call.startswith(prefix))


result, calls, unit, mounts = run("enable")
assert result.returncode == 0, result.stderr
assert "ExecStart=" in unit and "boot-launcher.sh" in unit and "WantedBy=multi-user.target" in unit
assert index(calls, "systemd-analyze verify") < index(calls, "mount -o remount,rw /"), "Validate the unit before changing the tablet"
assert any(call.startswith("umount -R") and call.endswith("unit-absent") for call in calls), "Write the unit only after /etc reaches the root file system"
assert index(calls, "umount -R") < index(calls, "systemctl enable ankimarkable-start.service")
assert index(calls, "systemctl enable") < index(calls, "systemd-run")
assert "/ ext4 ro" in mounts, "Return the root file system to read-only"
assert "Automatic startup is on" in result.stdout

result, calls, unit, mounts = run("enable", collapses=False)
assert result.returncode != 0 and "permanent" in result.stderr
assert unit is None, "A change that would vanish at restart must not be written"
assert not any(call.startswith(("systemctl", "systemd-run")) for call in calls)
assert "/ ext4 ro" in mounts

for changes in ({"firmware": "3.29.0.1"}, {"host": "imx8mm-ferrari"}):
    result, calls, unit, mounts = run("enable", **changes)
    assert result.returncode != 0
    assert not any(call.startswith(("mount", "umount", "systemctl", "systemd-run")) for call in calls), "Unsupported tablets stay untouched"
    assert unit is None

result, calls, unit, mounts = run("disable", firmware="3.29.0.1", installed=True)
assert result.returncode == 0, result.stderr
assert unit is None
assert index(calls, "umount -R") < index(calls, "systemctl disable ankimarkable-start.service") < index(calls, "systemd-run")
assert "Automatic startup is off" in result.stdout

print("Startup contracts passed: validated persistent unit, refused volatile writes, unsupported tablets untouched, clean disable, restart scheduled.")
