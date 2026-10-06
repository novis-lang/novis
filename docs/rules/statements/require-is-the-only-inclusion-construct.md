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
