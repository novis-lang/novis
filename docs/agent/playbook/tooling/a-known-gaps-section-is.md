- **A `# Known gaps` section is **one** item to `tools/owners.py`, however many paragraphs it has,
  so a second `— owner:` trailer inside it fails as `two owner tags in one item`.** The failure
  reads like a formatting quibble and is not: the block's *last* line is the tag, and a paragraph
  break does not start a new item, so a module that owes two unrelated things either names one owner
  for both or writes them as a numbered list. Run `python tools/owners.py --check
  --untagged-is-an-error` after editing any `# Known gaps` block — it names the file and line, and
  it is a floor check, so a stray trailer holds the whole run. [until: reviewed 2026-10-14]
