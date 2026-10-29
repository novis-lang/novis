When `Core` moves a name, code that still writes the old name does not compile, and the error says where
the name went. `Core\Order` became `Core\Arr::Order`, so `use Core\Order;` is `E0306`, and `Core\Order`
in a type or in an expression is `E0303`. Each of those errors gets a help line that names the new place
and one safe fix that writes it, which `nvs check --fix` applies and an editor offers as one action.

**One table lists every move.** `crates/nvs-stdlib/src/moved.rs` has one row per move: the old name, the
new name and the release that moved it. A test reads the registry and fails when a row's new name does
not exist or its old name still does, so the table cannot name a place that is not there. The pass that
reads the table, `nvs_types::moved`, runs over the finished diagnostics in `nvs check` and in the language
server, as the `autoload` note does, and leaves every other diagnostic as it was.

**The fix writes the new name in full.** In a type or an expression it replaces the old name where it is
written. A `use` line of a moved name is deleted, and every reference to the short name it brought in is
rewritten to the new name. A class-scoped enum such as `Core\Arr::Order` cannot be imported, so this is the
one rewrite that fits every move. An aliased or a grouped `use` gets the help line alone, because its short
name is not the old name's last segment.

**A `Core` class that does not exist is not declared.** `Core\Missing::Case` is `E0303`, the error the
same name gets in a type, and not `E0405` about a member of a class that is not there. The class is the
mistake, and one code lets the moved-name note find every place.

**A row stays until 1.0.** Before 1.0 any release may move a `Core` name
(`rule:core-api/no-deprecation-window-before-a-release`), and a project that skips several releases meets
every move at once. A row costs a few bytes in the binary and is read only for an `E0303` or an `E0306`, so
it costs nothing on the request path. At 1.0 the table is emptied, and the mechanism stays for the next
major version.

**Declined:**
- **An alias that keeps the old name working.** It is a second name for one thing
  (`rule:statements/nothing-gets-a-second-name`), and a window this project does not keep before a release.
- **A guess by edit distance across `Core`.** A guess may name the wrong thing, so it cannot be a fix that is
  applied without review, and the table already knows the answer.
- **The pass in `nvs-hir`.** That crate cannot read `nvs-stdlib`, so the table would need a second copy
  there.
