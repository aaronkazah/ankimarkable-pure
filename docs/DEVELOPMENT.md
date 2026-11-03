# Add a feature

The app has two parts: Anki manages cards and scheduling, and the tablet's native reMarkable editor manages handwriting. Use the native editor's existing tools for writing features.

## Find the right file

| Change | File |
|---|---|
| Card display, deck list, rating buttons | `frontend/Main.qml` |
| Native editor integration and answer pages | `frontend/NativeEditor.qml` |
| Anki actions and the state sent to the screen | `src/session.rs` |
| Anki collection, scheduling, sync | `src/backend.rs` |
| Card HTML, images, fonts, math | `src/render.rs`, `src/mathrender.rs` |
| Setup, update, import, export, diagnostics | `ankipure` |
| Read desktop Anki and its saved login | `scripts/desktop-anki.py` |

## Example: add a card action

1. Add a request variant in `src/session.rs`, with typed fields.
2. Handle it using Anki's backend and return the updated state.
3. Add a button or menu item in `frontend/Main.qml` that sends that action.
4. Add a focused test alongside the request handler.

Run the checks and install your change:

```sh
./ankipure test
./ankipure update
```

The CLI builds the source, backs up Anki data, installs the new files, and reopens the app. It does not commit or push code.

## Writing behavior

`NativeEditor.qml` opens the shared native scratchpad page. Before reveal, the question and Show answer button occupy one card box above a small writing area. Revealing saves and hides the native page while keeping the editor loaded, and expands that same box to show question and answer in one scroll. Both card images are prepared when a card opens. The study renderer omits Anki's `FrontSide` replacement because the question is already displayed; the public full-card HTML renderer retains the authored back template.

AppLoad's window wrapper excludes native pen input. Once the expected page has loaded, the integration places the complete native editor in a sibling layer immediately above the AppLoad window, preserving native toolbar exclusion regions. Its position tracks the writing area. Hiding or closing the app hides the layer; unloading destroys it. Keep this lifecycle intact: a focused, loaded editor inside the AppLoad wrapper can still have an empty pen region and silently discard strokes.

The integration keeps pen proximity live, registers the active document with the native document listener, and restores the toolbar position when leaving. New notebook pages receive a blank template before opening. Handwriting stays in the native editor without a second drawing implementation.

The card region uses the tablet's fast refresh mode and static button backgrounds. A new question is displayed before its native notes page opens, using the first frame swap without a delay timer. Set the horizontal toolbar preference before creating the native view. Readiness requires the expected page, completed native loading, drawing focus, and a shown horizontal toolbar with a nonzero height. Native page initialization may still request display cleanup.

The shared notebook has a permanent native text cover on page zero. Its identity is saved as soon as it is created, before adding the first card page. All decks and cards reuse one scratchpad page. Before exposing a new attempt, clear its native strokes and typed text once. Repeated state acknowledgments must never erase an in-progress response. `savePage()` completes deferred native work, including thumbnails, without changing the cover.

The shared notebook ID and scratchpad page ID are saved in `native-notes.json`. IDs are strings so large Anki card IDs keep their exact value. The Anki backend remains the authority for scheduling. Preview renders cards without submitting grades or changing their schedules. Due counts and total card counts are separate.

## Sync

`connect-anki` reads a consistent SQLite snapshot of the desktop collection, including committed WAL changes. It never opens the live desktop database for writing. The initial tablet copy is staged and backed up; repeated connection keeps tablet progress and updates only the saved sync token.

AnkiWeb or a configured Anki sync server is the authority for two-way merges. `sync-auth.json` stores a token and optional endpoint, with mode 600. Passwords are used only during login. Do not log or export credentials. Full downloads create a private recovery package before replacing local cards.

## Test without an account

The automated suite creates isolated sample collections and a localhost Anki sync server. It does not need a real Anki account. Installer and CLI contracts use an isolated fake device to check safe updates, backups, paths, and connection handling.

`tests/tst_workflow.qml` clicks through the production interface. `tests/workflow_backend.py` connects it to the compiled application using the real AppLoad socket protocol and inspects isolated collection snapshots. Add new user journeys here so the interface and scheduler are tested together. The workflow covers multiple-card progression and legitimate one-card learning repeats separately. Native handwriting is simulated in these desktop tests.

Check native editor changes on a Paper Pure running **3.28.0.172**. Start with the first stroke on a newly opened page, then check erasing, selection, undo/redo, layers, answer reveal, grading, and closing/reopening. Device acceptance is required for native UI changes because those components are supplied by the tablet.

## Release a change

```sh
./ankipure build
```

The installable archive and checksum appear in `dist/`. Review source, licenses, device checks, and documentation before publishing. Include corresponding source with redistributed builds. Keep accounts, collections, notebook IDs, device logs, and screenshots containing personal notes out of the repository.

The native editor is part of reMarkable's installed software. This repository references its on-device components and does not redistribute them. Firmware support is deliberately pinned; validate a new firmware version before changing the installer guard.

The launcher hook intercepts only the configured Anki notebook in the library. It launches the app before the normal document loader runs. Other notebooks keep their normal editor. The hook does not intercept the app's embedded DocumentView.
