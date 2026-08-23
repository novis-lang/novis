"""Replace one exact block of text in a file with another, both read from files.

CLAUDE.md forbids carrying file content through a shell, and the Bash tool
mangles a backslash inside a heredoc — which is every other line of Rust in this
repository. So a multi-line edit goes: write the old block and the new block to
files with the Write tool, then

    python .claude/splice.py <target> <old-file> <new-file>

It refuses anything but exactly one match, so a stale or ambiguous anchor is an
error rather than a silent wrong edit, and it writes with newline='' so a file's
existing line endings survive untouched.
"""

import sys

if len(sys.argv) != 4:
    print("usage: splice.py <target> <old-file> <new-file>")
    sys.exit(2)

target, old_path, new_path = sys.argv[1], sys.argv[2], sys.argv[3]
src = open(target, encoding="utf-8").read()
old = open(old_path, encoding="utf-8").read()
new = open(new_path, encoding="utf-8").read()
if src.count(old) != 1:
    print("anchor found %d times" % src.count(old))
    sys.exit(1)
open(target, "w", encoding="utf-8", newline="").write(src.replace(old, new))
print("spliced")
