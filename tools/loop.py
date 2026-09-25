"""The unattended loop is `bun nv loop`. This file runs it with the same arguments and exits with its
status, so a `respawn.py` that starts `python tools/loop.py` starts the new driver.

It runs `tools/nv/main.ts` itself rather than the `nv` package script: `bun run` prints the command
before it and an error line after every exit that is not 0, and a turn that asks for the next one exits
75. A Ctrl-C reaches the driver too, and the driver sweeps the session and ends the run; this process
waits for that rather than killing it half way.
"""

import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

proc = subprocess.Popen(["bun", os.path.join(ROOT, "tools", "nv", "main.ts"), "loop", *sys.argv[1:]], cwd=ROOT)
while True:
    try:
        sys.exit(proc.wait())
    except KeyboardInterrupt:
        continue
