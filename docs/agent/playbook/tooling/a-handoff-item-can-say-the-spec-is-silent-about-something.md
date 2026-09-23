- **A handoff item can say the spec is silent about something the spec declares in a fenced block.**
  `docs/spec/01-core-library.md`'s members table cites a type (`->type(): ColumnType`) without
  defining it, and the definition sits one screen down under *Enums, settings and errors*, so a
  reader who greps the table region concludes there is none. One `grep -rn <Name> docs/` is the
  whole check, before deciding that a slice gets to invent the cases. [until: reviewed 2026-09-06]
