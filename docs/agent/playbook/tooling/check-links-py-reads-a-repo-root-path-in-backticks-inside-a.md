- **`check-links.py` reads a repo-root path in backticks inside a `.py` docstring as a link, so
  deleting a tracked file turns a *tool's prose* red as well as a doc's.** Deleting the index
  `tools/owners.py:102` calls `CARRIED_GAPS` left that tool and `tools/playbook.py` passing every
  other gate while the link check reported `retired  tools/owners.py:102 -> …`, because both module
  docs spelled the full path while explaining why the file is gone. Run `python tools/check-links.py`
  in the same slice as any deletion, and in prose — a bullet here as much as a docstring — that has
  to go on mentioning the path, name the constant that holds it rather than spelling it.
  [until: gone tools/nv/cmd/links.ts:retired]
