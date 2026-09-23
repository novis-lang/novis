- **A blessed `.out` holding non-ASCII looks broken when the blesser echoes it back, and the file is
  fine.** `python tools/dossier.py --bless` prints what it wrote through the terminal's own code
  page, so on Windows `Café` comes back as `Caf?` and a Cyrillic line as a row of question marks
  while the `.out` on disk holds correct UTF-8. Read the file instead of the echo before deciding an
  example is wrong — `python -c "import sys; sys.stdout.write(ascii(open('<file>','rb').read().decode('utf-8')))"`.
  [until: reviewed 2026-09-18]
