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
- **An automatic once-guard as `require`'s only mode.** A template partial required in a loop would
  silently run once, trading one silent PHP behaviour for another.
