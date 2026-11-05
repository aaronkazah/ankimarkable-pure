# Testing

Run `./ankipure test`. Tests use temporary collections and a localhost sync server, without a real account or existing notes.

The automated checks cover:

- Seven interface-to-database workflow cases run the production QML, executable, AppLoad packet protocol, renderer, Anki scheduler, and collection together. They click Review, Show answer and all four ratings; verify black button pixels and loaded images; check exact saved ratings, next-card progression, double-tap protection, undo, reopening, one-card learning repeats, completion, preview navigation, and shared scratchpad reuse. A localhost test bridge replaces AppLoad; the native writing component is simulated, so these tests do not establish physical pen or display behavior.
- QML review states, preserved, editable handwriting on reveal, enabled ratings, and writing-layer geometry, visibility, page readiness, visible horizontal toolbar readiness, clearing strokes and typed text once per attempt, keeping the editor loaded during reveal, and one-time cover creation and a fixed cover after writing or browsing. Native dependencies are stubbed in these checks; device tests remain necessary.
- Typed native-frontend requests and AppLoad message framing, including invalid input.
- The configured library notebook launches Anki; unrelated notebooks keep their native editor.
- Exact card IDs, rendered card images, reveal-before-rating, grading, and undo.
- Collection persistence and current `.apkg` content/media round trips.
- Previewing future cards, Previous/Next navigation, rendered content, and unchanged scheduling.
- Typed-answer templates hide their input marker and show the correct field on the back.
- Separate study answers omit `FrontSide`, while the full-card HTML API preserves the authored back. Typed fronts must not repeat their correct answer in the study answer panel.
- Height-dependent card styles remain correct when reusing a document for its full-height image.
- Both card images are prepared before reveal; revealing and recording a notes page do not rewrite them.
- Consistent read-only desktop snapshots, media transfer, private cached sync tokens, and profile selection.
- Download and review upload through a real localhost Anki sync server, including cached-token reuse and recovery exports before full downloads.
- Installer staging: interrupted transfer preserves the previous app; successful update preserves data and migrates the sync endpoint.
- CLI connection reuse, backup-before-import, private backup permissions, safe filenames, device guards, and launcher requests.
- Automatic startup: the unit is validated before the tablet changes, written only after `/etc` reaches the root file system, refused while the overlay stays mounted, and removed cleanly. Unsupported tablets stay untouched.
- The math helper is found beside the running executable, so CLI card preparation renders math the same way as the app.

## Device acceptance

Use Paper Pure firmware **3.28.0.172** and AppLoad **0.6.0**. Native editor changes require on-device acceptance.

1. Open review and preview; check one card box, a smaller writing area, and the horizontal toolbar.
2. Open a new card and write the first stroke. It must register and appear immediately.
3. Confirm ink stays under the Marker, with palm rejection and the selected pen thickness.
4. Try eraser, selection/move, undo/redo, layers, and text tools.
5. Tap Show answer inside the question box. Confirm existing writing remains visible, add a correction, and check that the question and answer share one scroll above the writing area. Rating buttons must remain above the writing area, and only the next attempt clears it.
6. Check that the four ratings have distinct grayscale fills and correct intervals.
7. Write strokes on multiple layers and add typed text. Reveal and grade. The next attempt must start blank, with the same scratchpad page. Repeat the same learning card, then close and reopen; each attempt must start blank while review progress persists.
8. Review cards in two different decks, then close the app. Check that there is one **Anki** notebook with its cover and no additional pages per review. Tap the Anki notebook and confirm it opens the app directly.
9. Confirm normal reMarkable notebooks still open and write correctly after closing Anki.
10. Run `./ankipure enable-startup`. After the automatic restart and unlock, **Anki** must appear in the sidebar. Restart again from the power menu and confirm it returns. Run `./ankipure disable-startup`; after its restart, the tablet must start with the normal interface.

The current build passes the Rust collection, package, and localhost sync tests and 49 QML checks, including seven real interface-to-database workflows. They cover all ratings, progress, undo, same-card learning repeats, a shared scratchpad, a text-free loading indicator, and library shortcut routing.

On-device checks use an isolated collection and terminal-injected Marker events. They verify visible first strokes, clearing strokes across layers and typed text, retaining the editor through reveal, reuse of the same scratch page, and clearing on a real same-card Again repeat. Tapping the library notebook launches the app directly. Reopening the normal collection uses the same empty scratchpad without creating a page. Historical saved pages are checked against a private backup. Physical Marker latency, palm rejection, and black-flash counts remain manual acceptance checks.


AnkiWeb has not been tested with a real account; localhost sync is automated. Pixel-buffer captures cannot establish the number of physical black flashes. Keep screenshots containing personal notebooks and all account data out of shared test artifacts.

## Navigation regression checks

The interface checks include expanding and collapsing deck groups, tapping labels without opening a review, dragging from a study button without activating it, queuing a Decks request during card loading, and displaying a stalled-request notice without submitting a duplicate rating. The sidebar shortcut launches Anki directly. Startup tests cover an unsupported device, changed firmware, an already-running launcher, and rollback when startup fails.

## Prepared pages

Cache tests verify reuse across sessions, invalidation after card edits, persistent math rendering, and answers taller than 12,000 pixels. A full preparation run over 442 cards completed without changing scheduling or review records. On-device preview checks passed for question-to-Decks, answer-to-Decks, and a Decks request queued during loading. A terminal-injected native Marker stroke registered, and the next attempt cleared the scratchpad. The direct Anki sidebar entry is present and enabled. On the device, `enable-startup` wrote the unit to the root file system rather than the in-memory `/etc` layer; after the restart and unlock, the launcher started within 4 seconds. A restart from the power menu and `disable-startup` remain manual checks from step 10. Updating the device from 0.4.0 to 0.4.1 with `./ankipure setup` kept the existing collection, and its backup left out the card images (0.25 MB instead of 126 MB).

Writing-area size checks cover Compact (28%), Balanced (45%) and Large (65%), with reveal switching to Balanced while preserving the editor and its writing. On-device Marker checks verify that original writing survives reveal, correction strokes register separately, and the next card clears the page. Native scrolling changed the viewport without changing pages. Physical multi-touch gesture acceptance remains separate from invoking the native scroll action.
