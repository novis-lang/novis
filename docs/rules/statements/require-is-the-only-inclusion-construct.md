`require 'path.nvs';` is the only same-frame inclusion construct. It throws when the target is missing or
unparseable, and it executes every time control reaches it — there is no once-only guard.

```nvs
require 'partials/header.nvs';        // kept — throws if missing, runs every time
include 'partials/header.nvs';        // rejected
include_once 'lib/util.nvs';          // rejected
require_once 'lib/util.nvs';          // rejected
```

All four keep their token spellings in the lexer, so the diagnostic names the replacement precisely:
*Novis keeps exactly one inclusion construct; use `require` — it already throws on a missing file and runs
every time it is reached.*

**The path is known while compiling.** The file graph closes before the program runs
(`rule:programs/no-runtime-autoload`), so `require` loads only a path the compiler can read. That
path is literal text (`'...'` or `"..."` with no interpolation), a class constant declared in a file
the walk has already read, or `.` between two of those. A relative path resolves from the file the
`require` is written in (`rule:programs/relative-paths-resolve-from-their-file`). Every other path —
a variable, a call such as `Core\Path::thisDir() . '/autoload.nvs'`, an interpolation, a heredoc, an
`as` conversion, or a constant no file read so far declares — is `E0330` at the path, whose help
names `require './autoload.nvs';`. Such a site could load nothing and only ever throw. The one
`require` that still throws for a missing target at run time is a literal in a source with no
folder, such as standard input, which has no directory to resolve it from.

Throwing on failure is the failure path everything else takes (`rule:errors/propagation`); warn-and-continue
exists nowhere in the language. The repeat-guard axis is not needed either: declarations resolve by
namespace and through the per-path compiled-unit cache rather than by splice count, and a template partial
required from inside a loop must still run every time.

PHP spells one operation with four keywords across those two axes, which is the second name for one
behaviour `rule:statements/nothing-gets-a-second-name` rules out. The survivor is `require` because its
PHP meaning is already exactly the kept behaviour, so a PHP file that writes it needs no rewrite. The
other three are refused for the behaviour each names, not for being PHP-shaped.

**Declined:**
- **Keeping `include` without its `_once` form.** Warn-and-continue is the one thing `include` adds over
  `require`, and its name would lead a PHP reader to expect it.
- **Keeping `require_once` beside `require`.** That is still two names on the repeat-guard axis, which
  has no work to do once declarations resolve by namespace.
- **A new, shorter keyword such as `load`.** Every PHP file that writes `require` would need rewriting
  for no change in behaviour.
- **A computed path that compiles and throws when it is reached.** No file is loaded for it while
  compiling, so it can never work, and the error belongs where the path is written.
- **An automatic once-guard as `require`'s only mode.** A template partial required in a loop would
  silently run once, trading one silent PHP behaviour for another.
