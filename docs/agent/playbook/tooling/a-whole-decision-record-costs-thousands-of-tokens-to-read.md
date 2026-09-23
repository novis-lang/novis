- **A whole decision record costs thousands of tokens to read; one of its `###` sections costs a
  fraction.** `python tools/peek.py <file>:"## 4"`, or `sed -n` between the heading and the next
  one, never `cat`. The same goes for a long module: `grep -n` for the anchor first.
  [until: reviewed 2026-09-06]
