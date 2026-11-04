# Anki on reMarkable Paper Pure

Review your Anki flashcards on a reMarkable Paper Pure and answer them in your own handwriting. Write your answer under the question, tap **Show answer**, compare, and rate how well you remembered. Anki schedules the next review from your rating, just as desktop Anki does.

**Version 0.4 is a preview.** Reviewing, writing, erasing and saving have been checked on a Paper Pure. Pen latency and palm rejection still need more testing. Sync uses Anki's own sync code and passes automated tests against a local sync server, but hasn't yet been tried with a real AnkiWeb account. Desktop Anki keeps automatic backups, and **File → Create Backup** makes one on demand.

## What you need

- A reMarkable **Paper Pure** with software version **3.28.0.172**. The version is shown in **Settings → General → Paper Tablet → Software**. Other versions are refused, and nothing on the tablet is changed.
- **Developer Mode** turned on. Turning it on erases the tablet; see step 1 below.
- A Mac or Linux computer. Windows isn't supported.
- A USB-C cable that carries data, not just power.
- Internet on the computer during the first setup. Setup downloads the XOVI and AppLoad launchers from GitHub and checks them against known checksums.
- To sync with desktop Anki: an AnkiWeb account, Wi-Fi on the tablet, and Python 3 on the computer. On a Mac, if Terminal says Python 3 is missing, run `xcode-select --install`.

## Install

1. **Back up, then turn on Developer Mode.** Turning it on erases the tablet, so first sync your notebooks to your reMarkable account. Then go to **Settings → General → Paper Tablet → Software → Advanced → Developer Mode**. From then on, the tablet shows a developer-mode warning each time it starts. That's normal. See [reMarkable's guide](https://developer.remarkable.com/documentation/developer-mode).
2. **Download** the file ending in `-paper-pure.tar.gz` from the latest [release](https://github.com/aaronkazah/ankimarkable-pure/releases), and double-click it to extract the `ankimarkable-pure` folder.
3. **Connect** the tablet to the computer with the USB cable, and unlock the tablet.
4. **Open Terminal in that folder.** On a Mac, type `cd ` (with a space), drag the folder into the Terminal window, and press Return.
5. **Run setup:**

   ```sh
   ./ankipure setup
   ```

   - The first time, Terminal asks `Are you sure you want to continue connecting`. Type `yes` and press Return.
   - When asked for a password, type the tablet's SSH password. It's in **Settings → General → Help → About → Copyrights and licenses**, under **General information**. Nothing appears on screen while you type.
   - Setup takes a few minutes. The tablet's interface restarts while the launcher installs.

When setup finishes, Anki opens on the tablet with a small sample deck. To open it later, tap **Anki** in the tablet's sidebar.

To update, download the new release and run `./ankipure setup` from its folder. Your cards and progress are backed up first and kept.

### After the tablet restarts

Setup doesn't change how the tablet starts. After a restart, the tablet starts normally and Anki is hidden. Connect it and run `./ankipure start` to bring Anki back to the sidebar.

To have Anki start by itself instead, run this once:

```sh
./ankipure enable-startup
```

The tablet restarts to finish. Unlock it, and **Anki** appears in the sidebar. If the launcher fails to start, the tablet falls back to its normal interface. To turn automatic startup off, run `./ankipure disable-startup`.

Automatic startup only works with software version 3.28.0.172. If the tablet installs a software update, it starts normally, without Anki, until a release of this app supports the update.

## Put your cards on the tablet

### Link desktop Anki (recommended)

This copies your cards to the tablet and keeps both in sync through AnkiWeb.

1. In desktop Anki, sign into AnkiWeb and press **Sync**. If Anki asks, upload your desktop cards.
2. Connect the tablet and run:

   ```sh
   ./ankipure connect-anki
   ```

This backs up the tablet's Anki data, then copies your desktop cards and media to the tablet. It reuses desktop Anki's saved AnkiWeb login, so you don't type your AnkiWeb password. Desktop Anki itself is never changed. The tablet stores only a sync token, never your password.

- If desktop Anki has several profiles, choose one: `./ankipure connect-anki --profile "My profile"`. A tablet links to one profile and refuses to switch silently to another.
- Running `connect-anki` again refreshes the login and keeps the tablet's progress.
- If desktop Anki isn't signed into AnkiWeb, your cards still copy, but they won't sync. Sign in on desktop later and run `connect-anki` again, or sign in from Terminal with `./ankipure sync-setup`.
- If your Anki profiles are stored somewhere unusual, set `ANKI_BASE` to the folder that contains them.

### Import a deck file

Use this if you don't use AnkiWeb. In desktop Anki, choose **File → Export**, pick **Anki Deck Package (\*.apkg)**, and tick **Include scheduling information** and **Include media**. Then run:

```sh
./ankipure import "My Deck.apkg"
```

The deck is added to the tablet's cards, and Anki opens when it's ready. Deck files are good for moving cards, but they don't merge review progress the way AnkiWeb sync does.

### Card images

The tablet draws each card the first time you open it, which can take a moment. `import` draws every card in advance. To do the same for cards from `connect-anki` or `sync`, run `./ankipure prepare`. Cards that haven't changed are reused.

## Sync with AnkiWeb

The app has no sync button. You sync from the computer, and the tablet must be on Wi-Fi, because it talks to AnkiWeb directly.

1. Press **Sync** in desktop Anki.
2. Connect the tablet and run `./ankipure sync`.
3. Study on the tablet.
4. Run `./ankipure sync` again, then press **Sync** in desktop Anki.

Sometimes Anki needs a one-way full sync, for example after you change a note type on desktop. The tablet then replaces its cards with AnkiWeb's copy. Reviews it hadn't synced yet are saved in a recovery file on the tablet, in `/home/root/.ankimarkable-pure/.sync-backups/`, but they aren't merged. To avoid this, sync the tablet before you change note types. If AnkiWeb has no cards yet, the tablet asks you to upload from desktop Anki first.

To save the tablet's cards, media and progress as a file, run `./ankipure export`. The file appears in `exports/` beside `ankipure`. To choose the name and folder, give a path: `./ankipure export "Saved Deck.apkg"`. Open the file in desktop Anki with **File → Import**.

## Review your cards

Tap **Review due →** beside a deck.

1. Read the question and write your answer in the area below it. The toolbar is the tablet's own: pens, eraser, selection, layers, text, undo and redo. Drag with two fingers to move around the page.
2. Tap **Show answer →** in the question box. The answer appears under the question. Your writing stays, so you can add corrections.
3. Rate your first attempt, not the corrected one, with the buttons just above the writing area: **Again** if you got it wrong, otherwise **Hard**, **Good** or **Easy**. Each button shows when the card comes back.

**Saved · Next card** confirms your rating. **Saved · Same card: another learning step** means Anki wants another try soon. In a deck with only one card, it can come straight back.

- **Preview cards** opens any card, due or not, without changing its schedule. To grade cards, use **Review due →**.
- **+ / −** opens or closes a group of decks.
- **Card actions**, during review: undo the last rating, flag the card, hide it until tomorrow, or pause it.
- **Settings → Writing area size**: Compact, Balanced or Large. Showing an answer switches the size to Balanced, and later cards keep Balanced until you change it.

Your handwriting goes on one scratch page in a notebook called **Anki**, which appears in your library once you've opened a card. Every attempt starts on a blank page, so practice never adds pages. Handwriting isn't saved into Anki or graded for you. While the launcher is running, tapping the **Anki** notebook opens the app; otherwise it opens as an ordinary notebook.

The app follows your collection's scheduling settings, including FSRS. Edit cards in desktop Anki. Cards can't play audio or video or run JavaScript, and collapsible sections are always shown open.

The screen may flash black briefly when the app opens or a page loads. That's the e-ink display refreshing.

## Commands

Run each command in Terminal from the `ankimarkable-pure` folder, with the tablet connected by USB. `./ankipure help` lists them too.

| Command | What it does |
|---|---|
| `./ankipure setup` | Install or update the app from this folder |
| `./ankipure open` | Open Anki on the tablet |
| `./ankipure start` | Bring Anki back after the tablet restarts |
| `./ankipure enable-startup` | Start Anki automatically after restarts |
| `./ankipure disable-startup` | Stop starting Anki automatically |
| `./ankipure connect-anki` | Copy desktop Anki's cards and link its AnkiWeb login |
| `./ankipure sync` | Sync with AnkiWeb |
| `./ankipure sync-setup` | Sign into AnkiWeb from Terminal |
| `./ankipure import FILE.apkg` | Add a deck file |
| `./ankipure export [FILE.apkg]` | Save cards, media and progress as a deck file |
| `./ankipure prepare` | Draw every card's images in advance |
| `./ankipure backup` | Back up the tablet's Anki data |
| `./ankipure status` | Show what's installed and running |
| `./ankipure doctor` | Check the connection and build tools |
| `./ankipure unlock` | Enter the tablet's passcode from Terminal |
| `./ankipure logs` | Show recent app logs |

Each command asks for the tablet's password once. Commands that change Anki data back it up first, into `.backups/` beside `ankipure`. Backups never include your sync login. To use a different SSH address, put `--host root@ADDRESS` before the command.

## Troubleshooting

- **Tablet unavailable**: wake and unlock the tablet, and check that the cable carries data.
- **The tablet interface is waiting**: unlock the tablet, or run `./ankipure unlock` to type its passcode in Terminal. Then retry.
- **Paper Pure USB connection is absent** (on a Mac with a VPN): the USB connection isn't up, so the tablet's address would go through the VPN. The command stops rather than send your password there. Reconnect the cable and retry.
- **Unsupported tablet/firmware**: this release supports only a Paper Pure with software 3.28.0.172. Nothing was changed.
- **XOVI is already installed**: you already use XOVI, and setup won't replace it. Add AppLoad 0.6.0 to your XOVI, then run setup again.
- **REMOTE HOST IDENTIFICATION HAS CHANGED**: the tablet was reset, so its SSH key changed. Run `ssh-keygen -R 10.11.99.1`, then retry.
- **Anki is missing after a restart**: run `./ankipure start`, or turn on automatic startup.

## Remove Anki

1. Save your cards with `./ankipure export`.
2. If you turned on automatic startup, run `./ankipure disable-startup`. Otherwise, restart the tablet. Either way, it then starts normally, and Anki no longer loads.

The **Anki** notebook stays in your library as an ordinary notebook; delete it if you like. To delete the app's files too, run `ssh root@10.11.99.1 'rm -rf /home/root/xovi /home/root/.ankimarkable-pure'` after the restart. This also removes the XOVI launcher. Turning off Developer Mode resets the tablet and removes everything.

## Build from source

With Docker running, in a clone of this repository:

```sh
./ankipure test     # run the automated checks
./ankipure build    # create the installable archive in dist/
./ankipure update   # build, back up the tablet's Anki data, and install
```

See [Development](docs/DEVELOPMENT.md) and [Testing](docs/TESTING.md).

## How it works

The app runs through [AppLoad](https://github.com/asivery/rm-appload) on the [XOVI](https://github.com/asivery/xovi) launcher. It doesn't flash firmware or replace reMarkable's software, and it leaves your notebooks alone. Handwriting uses the tablet's own editor, which stays on the tablet; this project doesn't distribute it.

## License

**AGPL-3.0-or-later.** If you share a build, keep the license and attribution and include the corresponding source.

Built on [ankimarkable](https://github.com/abaj8494/ankimarkable), [Anki](https://github.com/ankitects/anki), AppLoad and XOVI. Math uses [MicroTeX](https://github.com/NanoMichael/MicroTeX) and [tinyxml2](https://github.com/leethomason/tinyxml2). Font and math licenses are bundled.
