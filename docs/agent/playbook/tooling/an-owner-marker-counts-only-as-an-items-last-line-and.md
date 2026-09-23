- **An `— owner:` marker counts only as an item's *last* line, and settled prose trailing under
  `# Known gaps` swallows it.** `owners.py` folds every line after a bullet into that bullet until
  the next one starts, so the two closing statements under `crates/nvs-types/src/error_lib.rs`'s
  block left the marker mid-item and the item stayed on `--untagged` with nothing said about why.
  Move that trailing prose above the heading — under this goal's § *Standing decisions* it is a
  decision rather than a gap — instead of hunting the marker's indentation.
  [until: gone tools/owners.py:the owner tag is not the item's last line]
