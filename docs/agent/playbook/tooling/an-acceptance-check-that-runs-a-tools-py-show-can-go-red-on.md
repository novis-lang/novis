- **An acceptance check that runs a `tools/*.py --show` can go red on the console's *code page* rather
  than on the tree, and the tell is a traceback ending in `UnicodeEncodeError`.** `rules.py` printed
  `core-classes/crypto-interop-tier`'s metadata and then died on the `‖` in its body, because a console
  here is cp1252 and that tool carried no `sys.stdout.reconfigure`, so a rule that was written and
  correct reported as unwritten work. Run a red check's own `argv` and read its *last* line before
  touching the tree; the repair is the three lines `tools/rules.py:445-450` now carries, in whichever
  tool prints the prose. [until: reviewed 2026-09-12]
