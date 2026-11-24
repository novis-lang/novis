**This rule holds from the first release after Pre-Alpha.** Until then the table of moves and the list of
removed attribute options are empty: an old name gets the error any unknown name gets, and the changelog's
*Breaking changes* section is the only record of what to write instead
(`rule:core-api/no-deprecation-window-before-a-release`). The mechanism below stays built and tested, so
the first release that moves a name adds its row and nothing else.

When `Core` moves a name, code that still writes the old name does not compile, and the error says where
the name went. If `Core\Sorting` became `Core\Arr::Order`, `use Core\Sorting;` is `E0306`, and
`Core\Sorting` in a type or in an expression is `E0303`. Each of those errors gets a help line that names
the new place and one safe fix that writes it, which `nvs check --fix` applies and an editor offers as one
action.

**A member moves too.** If `Core\Str::size` became `Core\Str::length`, a call to the old member is `E0405`,
and it gets the same help line and fix. The fix writes the new `Class::member` over the class and the
member as written, a short name a `use` brought in included, because the checker already resolved that
name to the old class. A member whose arguments changed as it moved gets the help line alone, and the help
gives the call to write as code. No fix can write that call, because the old arguments do not map onto the
new ones one to one. A member removed with no one replacement gets a help line that names every member to
choose from, and no fix, because the choice is the program's. A class-scoped type moved out of its class
is a member row whose new name is a type.

**One table lists every move.** `crates/nvs-stdlib/src/moved.rs` has one row per move: the old name, the
new name, the release that moved it, and for a member whose arguments changed the call to write. A test
reads the registry and fails when a row's new name, a type or a member, does not exist or its old name
still does, so the table cannot name a place that is not there. The pass that reads the table,
`nvs_types::moved`, runs over the finished diagnostics in `nvs check` and in the language server, as the
`autoload` note does, and leaves every other diagnostic as it was. `crates/nvs-types/tests/moved.rs` runs
the pass over made-up rows through `note_moves_in`, so it stays tested while the table is empty. A removed
attribute option has its own list, `REMOVED_OPTIONS` in `crates/nvs-types/src/attributes.rs`, whose help
branch a unit test feeds the same way.

**The fix writes the new name in full.** In a type or an expression it replaces the old name where it is
written. A `use` line of a moved name is deleted, and every reference to the short name it brought in is
rewritten to the new name. A class-scoped enum such as `Core\Arr::Order` cannot be imported, so this is the
one rewrite that fits every move. An aliased or a grouped `use` gets the help line alone, because its short
name is not the old name's last segment.

**A `Core` class that does not exist is not declared.** `Core\Missing::Case` is `E0303`, the error the
same name gets in a type, and not `E0405` about a member of a class that is not there. The class is the
mistake, and one code lets the moved-name note find every place.

**What it costs.** A row is a few bytes in the binary and is read only for an `E0303`, an `E0306` or an
`E0405`, so it costs nothing on the request path. An empty table costs one comparison per such error.

**Declined:**
- **An alias that keeps the old name working.** It is a second name for one thing
  (`rule:statements/nothing-gets-a-second-name`), and a window this project does not keep before a release.
- **A guess by edit distance across `Core`.** A guess may name the wrong thing, so it cannot be a fix that is
  applied without review, and the table already knows the answer.
- **The pass in `nvs-hir`.** That crate cannot read `nvs-stdlib`, so the table would need a second copy
  there.
- **Rows for the moves made while Pre-Alpha.** A list of old names is a surface to keep, and Pre-Alpha has
  no users to protect. The changelog records each move.
- **Deleting the mechanism while its table is empty.** The rule holds after Pre-Alpha, and building it
  again then costs more than keeping it tested now.
