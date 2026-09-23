- **`grep -r --include=<glob> .` through the Bash tool walks this tree and silently finds nothing**,
  returning exit 0 with no output, so a miss reads as a clean answer: it found 0 hits for a pattern
  ripgrep found across files it had just been pointed at by name. Use the Grep tool for any
  repo-wide question whose answer you are about to write down; a shell `grep -n` on one named file
  is still fine. [until: reviewed 2026-09-06]
