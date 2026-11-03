#!/usr/bin/env python3
"""Read a desktop Anki profile without changing its live collection."""

from __future__ import annotations

import argparse
import io
import json
import os
from pathlib import Path
import pickle
import shutil
import sqlite3
import sys


class ProfileReader(pickle.Unpickler):
    def find_class(self, module: str, name: str):
        if module.endswith("sip") and name in ("_unpickle_type", "_unpickle_enum"):
            return lambda *args: None
        raise ValueError("Unsupported desktop profile data")


def anki_folder() -> Path:
    if override := os.environ.get("ANKI_BASE"):
        return Path(override)
    if sys.platform == "darwin":
        return Path.home() / "Library/Application Support/Anki2"
    if sys.platform == "win32":
        return Path(os.environ["APPDATA"]) / "Anki2"
    return Path(os.environ.get("XDG_DATA_HOME", str(Path.home() / ".local/share"))) / "Anki2"


def snapshot(base: Path, output: Path, selected: str | None) -> None:
    with sqlite3.connect((base / "prefs21.db").resolve().as_uri() + "?mode=ro", uri=True) as preferences:
        rows = preferences.execute("select name,data from profiles where name != '_global'").fetchall()
    if selected is None:
        if len(rows) != 1:
            raise ValueError("Choose a profile: ./ankipure connect-anki --profile \"PROFILE NAME\"")
        selected = rows[0][0]
    data = next((data for name, data in rows if name == selected), None)
    if data is None:
        raise ValueError("That Anki profile was not found")
    profile = ProfileReader(io.BytesIO(data)).load()
    folder = (base / selected).resolve()
    if folder.parent != base.resolve():
        raise ValueError("Invalid Anki profile folder")
    output.mkdir(mode=0o700, parents=True, exist_ok=True)
    output.chmod(0o700)
    collection = output / "collection.anki2"
    with sqlite3.connect((folder / "collection.anki2").as_uri() + "?mode=ro", uri=True) as source:
        with sqlite3.connect(collection) as target:
            source.backup(target)
            count = target.execute("select count(*) from cards").fetchone()[0]
    collection.chmod(0o600)
    media = folder / "collection.media"
    if media.exists():
        shutil.copytree(media, output / "collection.media")
    else:
        (output / "collection.media").mkdir(mode=0o700)
    (output / "desktop-origin.json").write_text(json.dumps({"profile": selected}) + "\n")
    key = profile.get("syncKey")
    if key:
        auth = output / "sync-auth.json"
        with open(auth, "w", opener=lambda path, flags: os.open(path, flags, 0o600)) as file:
            json.dump({"hkey": key, "endpoint": profile.get("currentSyncUrl") or profile.get("customSyncUrl") or None}, file)
        print(f"{count} cards ready. Desktop sync login found.")
    else:
        print(f"{count} cards ready for offline use. For two-way sync, sign into AnkiWeb in desktop Anki, press Sync, then run ./ankipure connect-anki again.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--profile")
    arguments = parser.parse_args()
    try:
        snapshot(anki_folder(), arguments.output, arguments.profile)
    except (OSError, sqlite3.Error, ValueError, pickle.UnpicklingError) as error:
        parser.exit(1, f"Could not read desktop Anki: {error}\n")
