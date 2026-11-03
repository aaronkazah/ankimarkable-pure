# Anki on reMarkable Paper Pure

Read your Anki cards, write a response with the native reMarkable editor, show the answer, and rate your recall. The question and revealed answer share one card box. Revealing saves and hides the handwriting area, giving the card the full screen. The writing area has its native toolbar across the top and can be resized. **Preview cards** opens any card, including cards that are not due.

**0.4 preview:** the card layout, native input routing, visible test strokes, erasing, and saved handwriting have been checked on the supported tablet. Automated package transfer and local sync checks pass. Physical Marker latency and palm rejection still need acceptance.

## What the buttons mean

- **Review due →**: practise cards Anki schedules for today. Your Again/Hard/Good/Easy rating sets the next review.
- **Preview cards**: browse all cards without changing their schedule.
- **+ / −**: expand or collapse a deck group. Scrolling or tapping its name does not begin a review.
- **Show answer →**: reveal the correct answer, then grade your own recall. This is Anki's standard action; handwriting is not automatically marked.
- **Writing space**: choose Compact, Balanced, or Large. Your choice is saved, and resizing keeps your current writing.
- **Card actions**: undo the last review, flag a card, hide it until tomorrow, or pause its note. Available during review, not preview.

A **deck** is Anki's name for a collection of flashcards.

After a rating, **Saved · Next card** confirms progress. **Saved · Same card: another learning step** means Anki has scheduled another attempt at that card. This can happen immediately in a one-card deck because Anki brings nearby learning steps forward when no other cards are waiting. The rating is saved; use a parent deck to practise more cards together. Choose ratings based on your recall, not to force the card away.

## One handwriting notebook

All decks share one notebook named **Anki**. Tap it in your reMarkable library to open the flashcard app. Its permanent cover gives it a recognisable preview.

The writing area is a scratchpad: it starts blank for every new attempt, including a repeated learning card, switching decks, or reopening the app. All cards reuse the same page, so practice does not create more notebooks or pages. Drag with two fingers to pan within that page. Separate multi-page answers are not supported. Showing the answer hides the scratchpad; the next attempt clears it. Earlier saved handwriting from previous app versions remains in the notebook.

Flashcard questions, answers, and scheduling stay in Anki. The library file opens the app while AppLoad is running; after a stock restart it remains a normal notebook until you run `./ankipure start`.

## Install

Supported: **Paper Pure**, firmware **3.28.0.172**, Developer Mode, and USB SSH. Enabling Developer Mode for the first time resets the tablet; follow [reMarkable's instructions](https://developer.remarkable.com/documentation/developer-mode) and back up first.

1. Extract the installable archive from [Releases](https://github.com/aaronkazah/ankimarkable-pure/releases).
2. Connect the unlocked tablet with a USB data cable.
3. Open Terminal in the extracted folder and run:

```sh
./ankipure setup
```

On a Mac, you can type `cd ` in Terminal, drag the extracted folder into the window, and press Return to enter that folder.

Enter the tablet's SSH password when asked. It is under **Settings → Help → Copyrights and licenses**. A storage lock can be opened with `./ankipure unlock`; also unlock the tablet's on-screen passcode. The app opens automatically after installation. Later, use the tablet sidebar → **Anki**.

## Connect your desktop Anki

The easiest ongoing sync is **AnkiWeb**, using the same account on desktop and tablet.

1. In desktop Anki, press **Sync** and sign into AnkiWeb. Complete the first sync with your desktop cards uploaded.
2. With the tablet connected, run:

```sh
./ankipure connect-anki
```

The first connection copies a consistent snapshot of your desktop collection and media to the tablet, after backing up tablet data. It does not edit the live desktop collection. It also reads desktop Anki's saved sync login, so you do not type your AnkiWeb password again. Running the command again refreshes the login and keeps existing tablet review progress.

If desktop Anki has several profiles, choose one:

```sh
./ankipure connect-anki --profile "My profile"
```

This command requires Python 3 and finds Anki's standard profile folder on macOS or Linux. Set `ANKI_BASE` if your profiles are stored elsewhere. A tablet connects to one desktop profile; it will refuse to silently replace an existing connection with another profile.

Without an AnkiWeb login, the initial card copy still works offline. Sign into desktop Anki later and run `connect-anki` again to enable sync. You can also use `./ankipure sync-setup` to sign in privately in Terminal. Only a sync token is stored on the tablet, in a root-only file; the password is not saved.

## Keep Anki in the sidebar after a restart

After installing the current build, connect over USB and run:

```sh
./scripts/enable-startup.sh
```

This starts the app launcher after unlocking the tablet. It runs only on the supported firmware and restores the stock interface if the launcher fails. A firmware upgrade leaves the stock interface active until compatibility is checked again.

## Everyday sync

The tablet has no cloud-sync button. USB imports work without an account. If you configure AnkiWeb, use the Terminal command below to exchange edits and review progress, then sync desktop Anki. A USB import alone is not a two-way sync.

With USB connected, you can sync from Terminal too:

```sh
./ankipure sync
```

A folder of exported packages is useful for transferring decks, but it does not merge review progress and deletions like AnkiWeb. Use account sync for ongoing use. [Read Anki's sync guide](https://docs.ankiweb.net/manual/syncing).

A full download saves a recovery `.apkg` on the tablet under `.ankimarkable-pure/.sync-backups/` before replacing local cards. The client does not automatically upload a replacement collection. Resolve an upload-required full sync in desktop Anki.

## Import and export files

To import a deck without an account, use **File → Export** in desktop Anki, choose a packaged deck (`.apkg`), and include media and scheduling. Then run:

```sh
./ankipure import "My Deck.apkg"
./ankipure open
```

To export the tablet's cards, media, and review progress:

```sh
./ankipure export
```

The file appears in **exports/** beside the CLI. To choose another name or folder:

```sh
./ankipure export "Saved Deck.apkg"
```

Open the exported `.apkg` with desktop Anki to import it. Anki packages contain cards and scheduling; handwritten pages stay in native reMarkable notebooks. Use reMarkable's notebook export tools for those pages.

## Card preparation

Imports prepare both sides of every card before reopening the app. Existing prepared pages are reused when the card and its media have not changed. To prepare an existing collection manually, run `./ankipure prepare`. This keeps rendering work out of normal review. Cards use larger text in full-screen mode, and long answers can be scrolled to the end.

## Review or preview

Choose **Review due →** beside a deck to review its due cards. Read **QUESTION** and write in **YOUR HANDWRITTEN RESPONSE** below it. Press the black **Show answer →** button at the bottom of the question box. Your writing is saved and the writing area disappears. The same box expands to show the original question, a divider, and **ANSWER** in one scroll. Choose **Again**, **Hard**, **Good**, or **Easy** at the bottom, shown in four gray shades with the next-review intervals. The writing area returns for the next question.

If there are no due cards, the app says so and offers **Preview cards**. Preview also appears beside every deck. It shows every card with Previous/Next navigation and does not grade or change scheduling.

The toolbar edits your handwritten notes. Card text and templates are edited in desktop Anki. Native pen types, thickness, eraser, selection, layers, text, and undo/redo come from the tablet's own editor. The scratchpad lives in the single **Anki** notebook and clears when another attempt begins. Handwriting is not automatically inserted into Anki fields.

The menu includes undo last review, card flags, bury, and suspend. Anki controls scheduling, including your collection's FSRS settings. Audio/video playback, interactive JavaScript card templates, automatic handwriting grading, and card authoring are not implemented.

## Useful commands

| Task | Command |
|---|---|
| Install or update a downloaded bundle | `./ankipure setup` |
| Open the app | `./ankipure open` |
| Start AppLoad after reboot | `./ankipure start` |
| Check the tablet and installation | `./ankipure status` |
| Diagnose connection problems | `./ankipure doctor` |
| Back up tablet Anki data | `./ankipure backup` |
| Read app logs | `./ankipure logs` |

Backups are private files in **.backups/** beside the CLI, with sync credentials excluded. They contain Anki data and the scratchpad location. Back up saved notebooks with reMarkable's normal tools. The scratchpad is temporary.

Use `./ankipure --host root@TABLET_IP COMMAND` for a different SSH address. If the USB connection is absent, the CLI will refuse to send credentials through a VPN route. Wake, unlock, and reconnect the tablet, then retry.

## Build or add a feature

With Docker running, a source checkout can be tested and updated with:

```sh
./ankipure test
./ankipure update
```

To create a distribution without installing it, run `./ankipure build`. The package and checksum appear in **dist/**. See [Development](docs/DEVELOPMENT.md) and [Testing](docs/TESTING.md).

## Your software and notes

The app uses AppLoad and runtime UI additions. It does not flash firmware or replace the reMarkable executable. Existing notebooks are kept. Enable launcher startup using the instructions above to keep Anki available after a restart. Without it, run `./ankipure start` after restarting.

Brief black flashes can occur when the e-ink display refreshes during launch or when opening a native page. The app avoids reapplying identical screen states and adds no handwriting sleeps or batching. Physical refresh behavior must be checked on the supported tablet.

## License

**AGPL-3.0-or-later.** Keep the license and attribution and provide corresponding source with redistributed builds.

Built on [ankimarkable](https://github.com/abaj8494/ankimarkable), [Anki](https://github.com/ankitects/anki), [AppLoad](https://github.com/asivery/rm-appload), and [XOVI](https://github.com/asivery/xovi). Math uses [MicroTeX](https://github.com/NanoMichael/MicroTeX) and [tinyxml2](https://github.com/leethomason/tinyxml2). Font and math licenses are bundled. The proprietary reMarkable editor and assets stay on the tablet and are not distributed by this project.
