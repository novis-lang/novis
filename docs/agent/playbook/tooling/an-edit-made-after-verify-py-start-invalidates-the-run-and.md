- **An edit made after `verify.py --start` invalidates the run, and `fmt` is where you find out.**
  Each step reads the tree at whatever moment it runs, so a `cargo fmt` or a module-doc paragraph
  after `--start` gives a red verdict about a tree that no longer exists. There is nothing to debug:
  `--start` marks the end of editing, so `--start` again once the tree is final and only then write
  prose. [until: gone tools/verify.py:--start]
