- **`holes.py` only sees a numbered item whose bold title fits on one line, and a wrapped one fails
  *silently and backwards*.** `ITEM` is `^(\d+)\. \*\*(.+?)\*\*` without `re.DOTALL`, so a title
  that wraps before its closing stars matches nothing and, because an item's body runs to the *next*
  item mark, all of its anchors are absorbed by the item above — `--unattributed` drops to 0 and the
  gate goes green over a fiction. `python tools/holes.py --item N` is the check; it prints "no item
  N" for the item you just wrote. [until: gone tools/holes.py:(.+?)\*\*", re.MULTILINE]
