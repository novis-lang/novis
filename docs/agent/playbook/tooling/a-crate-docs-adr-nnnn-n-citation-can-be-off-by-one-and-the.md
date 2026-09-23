- **A crate doc's `ADR NNNN § N` citation can be off by one, and the handoff will copy it forward
  rather than check it.** A whole block of citations can sit one section high while the group line
  that inherits them reads plausibly. `python tools/peek.py <record>:"re:^### "` prints every
  heading in a few hundred bytes; run it before writing a doc comment that cites two or more
  sections of one record. [until: reviewed 2026-09-06]
