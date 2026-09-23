- **The line the driver quotes from a failed `dossier.py` check is often not the failure.** `dossier:
  target/release/nvs.exe is missing or older than the tree` is the routine note it prints on stderr
  *before* asking cargo for a current binary, so it heads the ledger entry while the real verdict sits
  two lines below it and says something else entirely. Re-run the check's own argv and read the last
  lines of its output, never the one line the ledger quotes.
  [until: gone tools/dossier.py:is missing or older than the tree]
