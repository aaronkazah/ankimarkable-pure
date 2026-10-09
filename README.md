# Anki for reMarkable Paper Pure

Review Anki flashcards on your Paper Pure and write answers with the Marker before revealing them. Ink stays visible with the answer, then clears when you grade the card.

This independent, open-source Anki client builds on Anki's backend and [ankimarkable](https://github.com/abaj8494/ankimarkable). It opens as a separate application through [AppLoad](https://github.com/asivery/rm-appload), preserving reMarkable OS and your reMarkable notebooks.

## Supported setup

- **reMarkable Paper Pure**, firmware **3.28.0.172**.
- Developer Mode enabled and USB SSH access working.
- XOVI with **AppLoad 0.6.0**.
- Portrait orientation; normal and inverted portrait are supported.

Paper Pro, Move, reMarkable 1/2, and other firmware versions are unsupported. The installer checks the model and firmware before copying files. Enabling Developer Mode itself factory-resets a tablet: back up first and follow [reMarkable's instructions](https://developer.remarkable.com/documentation/developer-mode).

## Install

Download the application archive and `SHA256SUMS` from [Releases](https://github.com/aaronkazah/ankimarkable-pure/releases). Verify its checksum, then extract it:

```sh
tar -xzf ankimarkable-pure-0.1.0-paper-pure.tar.gz
cd ankimarkable-pure
```

Connect the tablet with a USB data cable. Its generated SSH password is under Settings → Help → Copyrights and licenses. Test the connection:

```sh
ssh root@10.11.99.1
```

Keep an existing XOVI/AppLoad installation. If the launcher is missing, back up your notebooks and run:

```sh
./scripts/install-launcher.sh root@10.11.99.1
```

Install the application and ensure the launcher is running:

```sh
./scripts/install.sh root@10.11.99.1 .
./scripts/start-launcher.sh root@10.11.99.1
```

Open the tablet sidebar → **AppLoad** → **Anki — Paper Pure**. Fresh installations include a separate demo deck with text, an image, math, and a cloze card. An Anki account is optional.

Application files live under `/home/root/xovi/exthome/appload/ankimarkable-pure`. Cards and review progress live under `/home/root/.ankimarkable-pure`. Updates preserve the collection and your manifest configuration; close Anki before updating. The installer copies into a staging directory first, so an interrupted transfer leaves the previous app intact.

Launcher configuration is temporary. After reboot, reMarkable starts its normal interface; run `scripts/start-launcher.sh` when you want AppLoad. This project does not flash firmware or install a permanent boot override.

## Review cards

1. Choose a deck.
2. Write your answer in the card area.
3. Tap **Show Answer** to compare your work with the answer.
4. Choose **Again**, **Hard**, **Good**, or **Easy**. Anki schedules the next review.

The toolbar provides Home, Undo, Eraser, Clear, Sync, and Exit. The Marker Plus eraser works too. Use two fingers to scroll long cards or pinch to change text size. The card menu lets you flag, bury, or suspend it. Close with **Exit** or AppLoad's swipe-down gesture.

Scheduling uses Anki's backend, including FSRS when enabled in your collection. MathJax-style math and pre-rendered LaTeX SVGs are supported; releases bundle the math renderer and fonts. Sound playback, video, interactive JavaScript templates, and card authoring are not implemented.

## Import without an account

In Anki desktop, export a deck as `.apkg`, including media and scheduling to retain progress. Close Anki on the tablet before importing:

```sh
scp -O MyDeck.apkg root@10.11.99.1:/home/root/.ankimarkable-pure/import.apkg
ssh root@10.11.99.1 \
  '/home/root/xovi/exthome/appload/ankimarkable-pure/ankimarkable-pure-tools import /home/root/.ankimarkable-pure/collection.anki2 /home/root/.ankimarkable-pure/import.apkg'
```

Reopen the app to review. The tools also support `export COLLECTION PACKAGE.apkg`, `check COLLECTION`, and `preview COLLECTION DIRECTORY`.

## Optional sync

Back up the local Anki collection before connecting an account: an initial full download can replace the tablet's local cards. Sync does not touch reMarkable notebooks.

```sh
./scripts/configure-sync.sh root@10.11.99.1
```

The script prompts locally for your Anki username/password and saves them on the tablet in a root-only directory, in a file with permissions `600`. The password is plaintext inside the encrypted home partition; keep this file out of repositories and shared backups. Open Anki and tap **Sync** after configuration.

AnkiWeb access by this independent client is unofficial and may be withdrawn. A self-hosted Anki sync server is the tested alternative. Add `ANKIMARKABLE_SYNC_ENDPOINT` to the `environment` object in `external.manifest.json`, using the server's base URL with a trailing slash, such as `https://anki.example.com/`. Configure that server's username/password with the same script, then restart the app.

The client downloads when a full sync is required and permitted. Resolve an upload-only full sync in official Anki desktop first; this client never automatically performs a full upload.

## Return to the stock interface

Close Anki, then run:

```sh
ssh root@10.11.99.1 /home/root/xovi/stock
```

The Anki collection stays on the tablet. A reboot also returns to stock. To remove this app, close it and remove its directory under `xovi/exthome/appload`; keep `.ankimarkable-pure` to preserve cards and progress.

## Build from source

Install Docker with its engine running, then clone the source repository:

```sh
git clone https://github.com/aaronkazah/ankimarkable-pure.git
cd ankimarkable-pure
./scripts/test.sh
./scripts/build.sh
./scripts/install.sh root@10.11.99.1
```

The build uses ARM64 Linux containers, Rust 1.95.0, and the matching official Paper Pure SDK. Intel/x86 hosts need Docker's ARM64 emulation. Allocate at least 8 GB to Docker. The first build downloads dependencies and the SDK; later builds reuse caches. No host Xcode changes are needed.

Outputs are `dist/ankimarkable-pure-0.1.0-paper-pure.tar.gz` and `dist/SHA256SUMS`. SDK, launcher, math sources, and Rust dependencies are pinned. See [testing](docs/TESTING.md) for coverage and device checks.

## License and attribution

**AGPL-3.0-or-later**, inherited from ankimarkable and the Anki backend. Retain the license and attribution with redistributed builds and provide the corresponding source and build instructions.

- [ankimarkable](https://github.com/abaj8494/ankimarkable), by abaj8494: original application and pen whiteboard.
- [Anki](https://github.com/ankitects/anki): collection, scheduling, import/export, and sync backend.
- [AppLoad](https://github.com/asivery/rm-appload) and [XOVI](https://github.com/asivery/xovi), by asivery: launcher and display interface.
- [MicroTeX](https://github.com/NanoMichael/MicroTeX) and [tinyxml2](https://github.com/leethomason/tinyxml2): math rendering, MIT and zlib licenses respectively.
- Noto, XITS, and Nerd Fonts: licenses are in `assets/licenses` and the release's `licenses` folder.

This project is not affiliated with or endorsed by reMarkable or Ankitects.
