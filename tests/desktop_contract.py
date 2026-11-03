from pathlib import Path
import contextlib
import importlib.util
import io
import json
import pickle
import sqlite3
import tempfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("desktop_anki", ROOT / "scripts/desktop-anki.py")
desktop = importlib.util.module_from_spec(spec)
spec.loader.exec_module(desktop)

with tempfile.TemporaryDirectory(prefix="ankipure-desktop-") as temporary:
    base = Path(temporary) / "Anki2"
    profile = base / "Study"
    profile.mkdir(parents=True)
    preferences = sqlite3.connect(base / "prefs21.db")
    preferences.execute("create table profiles(name text,data blob)")
    preferences.execute("insert into profiles values(?,?)", ("Study", pickle.dumps({"syncKey": "test-private-key", "currentSyncUrl": "https://sync.test/"})))
    preferences.commit()
    live = sqlite3.connect(profile / "collection.anki2")
    live.execute("pragma journal_mode=wal")
    live.execute("create table cards(id integer primary key)")
    live.executemany("insert into cards values(?)", [(1,), (2,)])
    live.commit()
    media = profile / "collection.media"
    media.mkdir()
    (media / "diagram.png").write_bytes(b"image")
    original = (profile / "collection.anki2").read_bytes()
    output = Path(temporary) / "snapshot"
    stdout = io.StringIO()
    with contextlib.redirect_stdout(stdout):
        desktop.snapshot(base, output, None)
    assert "test-private-key" not in stdout.getvalue()
    assert (profile / "collection.anki2").read_bytes() == original
    with sqlite3.connect(output / "collection.anki2") as copied:
        assert copied.execute("select count(*) from cards").fetchone()[0] == 2, "Snapshot must include committed WAL changes"
    assert (output / "collection.media/diagram.png").read_bytes() == b"image"
    assert json.loads((output / "sync-auth.json").read_text()) == {"hkey": "test-private-key", "endpoint": "https://sync.test/"}
    assert (output / "sync-auth.json").stat().st_mode & 0o777 == 0o600
    assert output.stat().st_mode & 0o777 == 0o700
    assert json.loads((output / "desktop-origin.json").read_text())["profile"] == "Study"
    preferences.execute("insert into profiles values(?,?)", ("Other", pickle.dumps({})))
    preferences.commit()
    try:
        desktop.snapshot(base, Path(temporary) / "ambiguous", None)
        raise AssertionError("Multiple profiles require a choice")
    except ValueError:
        pass
    live.close()
    preferences.close()

print("Desktop contracts passed: consistent read-only snapshot, media, private cached login, and profile selection.")
