- **A control byte written through the Write or Edit tool does not reach the file, and nothing says
  so.** An escape or bell byte in the content is dropped silently, so an attack meant to carry
  terminal escapes lands as plain text and still passes. Write a placeholder such as `@ESC@`, swap it
  with a short `bun` script under `.agent-tmp/`, and confirm with `grep -c $'\x1b' <file>`.
