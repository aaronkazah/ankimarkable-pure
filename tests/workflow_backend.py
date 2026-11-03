"""Run the real application behind a localhost replacement for AppLoad."""

from contextlib import closing, contextmanager
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
import json
import os
import shutil
import socket
import sqlite3
import struct
import subprocess
import threading
import tempfile


class WorkflowBackend:
    def __init__(self, binary: str, directory: Path):
        self.binary = binary
        self.directory = directory
        self.data = directory / "collection"
        self.baseline = directory / "baseline"
        self.process = None
        self.start()
        self.stop()
        shutil.copytree(self.data, self.baseline)

    def start(self):
        path = self.directory / "app.sock"
        path.unlink(missing_ok=True)
        with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as listener:
            listener.bind(str(path))
            listener.listen(1)
            listener.settimeout(30)
            self.process = subprocess.Popen(
                [self.binary, str(path)],
                env=os.environ | {"ANKIMARKABLE_DATA_DIR": str(self.data)},
                stdout=subprocess.DEVNULL,
            )
            self.connection, _ = listener.accept()
            self.connection.settimeout(30)

    def stop(self):
        if self.process is not None:
            self.connection.sendall(struct.pack("<ii", -1, 0))
            self.connection.close()
            self.process.wait(timeout=30)
            assert self.process.returncode == 0
            self.process = None

    def reset(self, scenario: str):
        self.stop()
        shutil.rmtree(self.data)
        shutil.copytree(self.baseline, self.data)
        with closing(sqlite3.connect(self.data / "collection.anki2")) as db:
            with db:
                if scenario == "single":
                    db.execute("delete from cards where id != (select min(id) from cards)")
                elif scenario == "future":
                    db.execute("update cards set queue=2,type=2,due=100000")
                else:
                    assert scenario == "multiple"
        self.start()

    def request(self, request: dict):
        action = request["action"]
        if action == "_reset":
            self.reset(request["scenario"])
            return {}
        if action == "_restart":
            self.stop()
            self.start()
            return {}
        if action == "_inspect":
            # The single request worker leaves Anki idle while copying its locked DB.
            with tempfile.TemporaryDirectory(dir=self.directory) as snapshot:
                for suffix in ("", "-wal"):
                    source = self.data / f"collection.anki2{suffix}"
                    if source.exists():
                        shutil.copyfile(source, Path(snapshot) / source.name)
                with closing(sqlite3.connect(Path(snapshot) / "collection.anki2")) as db:
                    reviews = db.execute("select cast(cid as text),ease from revlog order by id").fetchall()
                    cards = db.execute("select cast(id as text),queue,due,reps from cards order by id").fetchall()
            notes_path = self.data / "native-notes.json"
            notes = json.loads(notes_path.read_text()) if notes_path.exists() else {}
            return {"reviews": reviews, "cards": cards, "notes": notes}
        payload = json.dumps(request).encode()
        self.connection.sendall(struct.pack("<ii", 1, len(payload)))
        self.connection.sendall(payload)
        kind, length = struct.unpack("<ii", self.connection.recv(8))
        assert kind == 101
        response = self.connection.recv(length)
        assert len(response) == length
        return json.loads(response)


@contextmanager
def workflow_server(binary: str, directory: Path):
    backend = WorkflowBackend(binary, directory)

    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            try:
                body = json.dumps(backend.request(request)).encode()
                self.send_response(200)
            except Exception as error:
                body = json.dumps({"error": str(error)}).encode()
                self.send_response(500)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):
            pass

    server = HTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}"
    finally:
        server.shutdown()
        thread.join()
        server.server_close()
        backend.stop()
