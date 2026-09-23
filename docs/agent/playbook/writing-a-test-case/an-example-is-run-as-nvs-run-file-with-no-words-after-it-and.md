- **An example is run as `nvs run <file>` with no words after it and no terminal, so a feature whose
  subject is input has to print something when it is given none.** `dossier.py` runs and blesses an
  example with a bare argument vector and captured streams, and there is no `--ARGS--` section like
  the one a `.nvst` case has. Write the program the way a real tool behaves with nothing given: an
  empty list from `Core\Cli::arguments`, a prompt taking its `{default: …}`, a `colorDepth()` of
  `None`.
  [until: reviewed 2026-09-21]
