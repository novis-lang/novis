- **An example that logs writes its own path into its `.out`, so it only matches when the file is
  named the way `dossier.py` names it.** A log record carries the file the program was started
  with, so an absolute Windows path froze an output no Linux run could reproduce — `--bless` and
  the sweep now both name an example repo-relative and posix, from the repository root. Check a
  suspect example with `python tools/dossier.py --run examples --only '<feature>'` rather than with
  a bare `nvs run` from wherever you are standing.
  [until: gone docs/examples/types/Log-Level/01-the-five-levels-and-what-each-is-for.out:source]
