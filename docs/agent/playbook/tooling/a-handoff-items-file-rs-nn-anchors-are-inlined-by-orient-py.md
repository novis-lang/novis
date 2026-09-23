- **A handoff item's `file.rs:NN` anchors are inlined by `orient.py`, so a stale one arrives as code
  that does not match the item's prose.** The line numbers were written before the previous
  session's own edits moved them, and the pack prints whatever now sits at the number under the
  heading "the code your item anchors". When a printed window does not match the item, do not read
  around the number: `python tools/peek.py --locate <symbol>` or a `:re:` target lands first time.
  [until: gone tools/orient.py:ANCHOR_RE]
