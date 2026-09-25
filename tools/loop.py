"""The unattended loop is `bun nv loop`. This file runs it with the same arguments and exits with its
status, so a `respawn.py` that starts `python tools/loop.py` starts the new driver.
"""

import subprocess
import sys

sys.exit(subprocess.call(["bun", "nv", "loop", *sys.argv[1:]]))
