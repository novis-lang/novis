While Novis is Pre-Alpha, a name `Core` removes from its own API is gone in the release that removes it:
a member, a parameter, an attribute option or an option of an options bag. Nothing keeps the old name
working beside the new one, and nothing warns before it goes. Code that still writes a removed name does
not compile.

**The error names the replacement.** A removed name is reported with the error any unknown name gets, and
its help says what to write instead, as code where there is code to write. For an attribute option that
help comes from one list in the checker of removed options and their replacements. A row stays on that
list while code written before the removal may still be compiled.

The window costs more than it buys before there are users to protect. Every kept name is a second
spelling a reader meets and a second path every later change must keep working, against *Simplicity*.
A removed name is never a silent change either: the build stops at the line, and the help line says what
to write, so the cost of a removal is one edit the error already describes.

This is about `Core`'s own API. A program or a package deprecates its own declarations with
`#[Core\Deprecated]` (`rule:attributes/a-deprecation-names-its-replacement-as-code`), and nothing here
changes that. The first release that promises stability replaces this rule with the window it promises.

**Declined:**
- **A deprecation window for `Core`.** Keeping a removed name working for a release adds a second
  spelling and a warning path to every removal, and Pre-Alpha code has no stability promise to keep.
- **A warning that names the replacement while the old name still works.** It is the window with a
  shorter life, and a warning is read less often than an error.
- **A bare unknown-name error with no replacement.** The reader then has to search the changelog for a
  name the checker already knows.
