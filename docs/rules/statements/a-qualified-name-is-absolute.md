Given a name as written, and the namespace and import set active where it was written:

- **Contains a separator** — it is the fully-qualified name, read from the root. The enclosing namespace
  and the import set are not consulted at all, **not even for its first segment**, which is what makes a
  namespace-prefix import (`use Core\Json;` then `Json\Derive`) name nothing.
- **Contains none** — look it up in the import set, then in the enclosing namespace, and nowhere else
  (`rule:statements/no-fallback-to-the-root-namespace`).

A `use` therefore does exactly one thing: it binds **one declaration** under its own short name. Importing
a *namespace* is not a concept here.

```nvs
namespace App;

use App\Models;           // binds the name `Models`, and nothing under it
use App\Models\User;      // a `use` path was always absolute

new User();               // short: found in the imports
new App\Models\User();    // absolute — the same name from any namespace
new Models\User();        // refused: it names `Models\User`, which does not exist
```

`E0322` is the migration diagnostic, firing on exactly the construct this changes: *`Models\User` does not
exist; a qualified name is absolute here. Did you mean `App\Models\User`?* A short name for a class in the
same namespace, every `use` path, and `Core\Str::length($s)` are unaffected.
