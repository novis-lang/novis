- **A character a session cannot type — a private-use code point, an unusual glyph — can reach a file
  through `Write` and then match no `old_string` you write afterwards.** The character is dropped from
  the later call, so the one line needing a fix is the one line that cannot be addressed. Write such a
  character as code — `String.fromCodePoint(0xe000)` — from the start, and if a raw one is already on
  disk, `git restore <file>` and redo the edit rather than hunting for an anchor.
  [until: gone AGENTS.md:A shell never carries file content]
