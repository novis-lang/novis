While Novis is Pre-Alpha, a name `Core` removes from its own API is gone in the release that removes it:
a member, a parameter, an attribute option or an option of an options bag. Nothing keeps the old name
working beside the new one, and nothing warns before it goes. Code that still writes a removed name does
not compile.

**An old name gets the unknown-name error, and the changelog says what to write instead.** A removed or
renamed name is reported with the error any unknown name gets (`E0303`, `E0306`, `E0405` or the
unknown-option error), with the ordinary closest-name suggestion where one exists. No help line names what
replaced it. The commit that removes or renames a name carries a `BREAKING CHANGE:` body that lists each
old name and the new one, and the changelog's *Breaking changes* section shows the code to write
(`docs/agent/changelog-summary.md`). The tables that would name a replacement,
`rule:core-api/a-moved-name-says-where-it-went`'s, stay empty until the first release after Pre-Alpha.

The window costs more than it buys before there are users to protect. Every kept name is a second
spelling a reader meets and a second path every later change must keep working, against *Simplicity*.
A list of old names and their replacements is the same cost in a smaller form: a surface the checker
keeps and every rename extends. A removed name is never a silent change either: the build stops at the
line, and the changelog says what to write.

This is about `Core`'s own API. A program or a package deprecates its own declarations with
`#[Core\Deprecated]` (`rule:attributes/a-deprecation-names-its-replacement-as-code`), and nothing here
changes that. The first release that promises stability replaces this rule with the window it promises,
and from then on a moved name's error says where it went.

**Declined:**
- **A deprecation window for `Core`.** Keeping a removed name working for a release adds a second
  spelling and a warning path to every removal, and Pre-Alpha code has no stability promise to keep.
- **A warning that names the replacement while the old name still works.** It is the window with a
  shorter life, and a warning is read less often than an error.
- **A help line that names the replacement while Pre-Alpha.** It needs a row per old name, kept until
  some later release, and Pre-Alpha has no users to protect. The changelog already records each rename.
