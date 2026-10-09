from pathlib import Path
import os
import subprocess
import tempfile


def check_update(interrupted: bool) -> None:
    with tempfile.TemporaryDirectory(prefix="pure-install-contract-") as temporary:
        base = Path(temporary)
        device = base / "device"
        app = device / "xovi/exthome/appload/ankimarkable-pure"
        app.mkdir(parents=True)
        (device / "xovi/extensions.d").mkdir()
        (device / "xovi/extensions.d/appload.so").write_text("launcher")
        (device / ".ankimarkable-pure").mkdir()
        collection = device / ".ankimarkable-pure/collection.anki2"
        collection.write_bytes(b"existing collection and review progress")
        original = b"previous working application"
        (app / "ankimarkable-pure").write_bytes(original)
        configuration = b'{"environment":{"ANKIMARKABLE_SYNC_ENDPOINT":"https://example.test/"}}'
        (app / "external.manifest.json").write_bytes(configuration)

        bundle = base / "bundle"
        bundle.mkdir()
        for name in ["ankimarkable-pure", "mathpng", "external.manifest.json", "icon.png", "LICENSE"]:
            (bundle / name).write_text("new package content")
        (bundle / "ankimarkable-pure-tools").write_text("#!/bin/sh\nexit 0\n")

        commands = base / "commands"
        commands.mkdir()
        (commands / "ssh").write_text("""#!/bin/bash
while [[ $# -gt 0 ]]; do
  case "$1" in -o|-S) shift 2 ;; *) shift; break ;; esac
done
if [[ "$*" == *'printf "%s|%s|"'* ]]; then
  printf 'imx93-tatsu|aarch64|3.28.0.172'; exit 0
fi
if [[ ${FAIL_COPY:-0} == 1 && "$*" == tar* ]]; then exit 9; fi
exec /bin/sh -c "$*"
""")
        (commands / "chown").write_text("#!/bin/sh\nexit 0\n")
        for command in commands.iterdir():
            command.chmod(0o755)
        source = Path(__file__).resolve().parents[1] / "scripts/install.sh"
        installer = base / "install.sh"
        installer.write_text(source.read_text().replace("/home/root", str(device)))
        environment = os.environ.copy()
        environment["PATH"] = f"{commands}:{environment['PATH']}"
        environment["FAIL_COPY"] = "1" if interrupted else "0"
        environment.pop("SSH_CONTROL_PATH", None)
        result = subprocess.run(["bash", str(installer), "root@test-device", str(bundle)], env=environment, capture_output=True, text=True)
        assert collection.read_bytes() == b"existing collection and review progress"
        assert (app / "external.manifest.json").read_bytes() == configuration
        if interrupted:
            assert result.returncode != 0
            assert (app / "ankimarkable-pure").read_bytes() == original
        else:
            assert result.returncode == 0, result.stderr
            assert (app / "ankimarkable-pure").read_text() == "new package content"
            assert not list((device / ".cache").glob("*.previous"))


check_update(interrupted=True)
check_update(interrupted=False)
print("Installer contracts passed: interrupted copy preserves the working app; successful update preserves configuration and progress.")
