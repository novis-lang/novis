The concealment is cosmetic, and what it cannot cover is written down and shipped in the tool's own
documentation, because a redaction trusted past its reach is worse than none.

Workspace search results and quick-open previews render matching lines outside any editor, so no
decoration applies. Diff and version-control views can be decorated, but there is no type information
for the "before" side, so the old value of an edited secret is visible in the review of that edit. The
minimap renders from the buffer. Any other extension's hover, lens or webview reads the document text
directly. A copy of a concealed range copies the plaintext, and nothing may intercept the clipboard.
And the file itself is on disk, in the working tree, and in the history the moment it is committed —
concealing a hardcoded credential does not make it less hardcoded.

This list is part of the decision, not commentary on it, and it is the real cost of the feature: a user
who believes the editor protects them everywhere is worse off than one who knows it does not.

**Declined:**

- **Intercepting copy** by rebinding the editor's copy action. It overrides a global keybinding for the
  whole editor to protect a value that is on disk anyway, and a redaction that fights the user's own
  clipboard is past the threat of an incidental viewer.
- **Refusing or warning on a `secret`-typed source literal in `nvs check`**, the cause the concealment
  only treats. Does not exist: the checker holds both facts it would need, but a test fixture, an example
  and a conformance case all legitimately write one.
- **Redacting the "before" side of a diff.** Does not exist: it needs a type answer for a revision of the
  document the server never analysed.

**Built:** the concealment itself (`rule:ide/redaction-ranges-come-from-the-server`). **Missing:** this
list in `editors/vscode/README.md`, which describes the concealment and not where it stops.
