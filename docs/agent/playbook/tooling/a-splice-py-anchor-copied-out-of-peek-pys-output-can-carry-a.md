- **A `nv splice` anchor copied out of `peek.py`'s output can carry a line break `peek` added.**
  `peek` wraps a long prose line for display, so a two-line anchor taken from a `.md` file may be
  one line on disk, and the refusal reads *"the anchor matches for its first 100 character(s) … the
  anchor wants: ''"*. Keep a prose anchor inside one displayed line or take it from `grep -n`;
  `nv splice` matches exactly, trailing newline included, so strip the one the Write tool ends a
  patch file with when splicing mid-paragraph. [until: reviewed 2026-09-06]
