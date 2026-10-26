A screenshot on a website page is taken from a real run by `bun nv site --shoot`, and nothing else
writes one. A hand-written `.shot` names an app under `apps/`, the requests to send it, and the steps
to take in the browser: open an address, click, type and wait. The command serves the app in
development with `[debug] store` on, so the server starts `nvs debug`, sends the requests, and takes
the steps in the Chromium headless shell Playwright pins, over the DevTools protocol. It writes the
picture beside the `.shot` as a `.png`, with a `.shot.out`.

The `.shot.out` is what the picture shows, as text: the page's visible text with every duration, date,
trace id and port masked, and the hash of the viewer's built assets. `bun nv site --check shots` runs
the same steps and fails when the text or the hash differs. Pixels are never compared, because two
machines draw the same page with different fonts and different anti-aliasing. A change to the viewer's
look changes the hash, so the picture is taken again.

A profile's picture is the speedscope build the VS Code extension bundles, opening a real
`--profile=speedscope` export of a fixture. Speedscope is the viewer a profile is handed to
(`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`), so the website draws no flame
chart of its own. Its `.shot.out` holds the frames and their call counts, never their times, because
times differ on every run.

- **Memory:** nothing in the language or the binary. The site carries each picture's file. `--shoot`
  and `--check shots` start a server, `nvs debug` and a browser once per run.

**Declined:**
- **Comparing pixels.** The same page differs by font rendering on each machine, so the check would
  fail on a machine that changed nothing.
- **A screenshot taken by hand.** Nothing would notice when the viewer changes.
