The concealment is cosmetic, and what it cannot cover is written down and shipped in the tool's own
documentation, because a redaction trusted past its reach is worse than none.

Workspace search results and quick-open previews render matching lines outside any editor, so no
decoration applies. Diff and version-control views can be decorated, but there is no type information
for the "before" side, so the old value of an edited secret is visible in the review of that edit. The
minimap renders from the buffer. Any other extension's hover, lens or webview reads the document text
directly. A copy of a concealed range copies the plaintext, and nothing may intercept the clipboard.
And the file itself is on disk, in the working tree, and in the history the moment it is committed —
concealing a hardcoded credential does not make it less hardcoded.

This list is part of the decision, not commentary on it.

**Not on disk.** There is no language server in the tree.
