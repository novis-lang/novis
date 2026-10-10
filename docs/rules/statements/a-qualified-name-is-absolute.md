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
same namespace, every `use` path, and `Core\Str::length($s)` are unaffected. A qualified name that
resolves neither way is the ordinary unresolved-name error.

The rule is decidable from the name alone, by whether it contains a separator, without knowing where the
name sits. PHP reads a qualified name relative to the current namespace, so inside `namespace App;` the
name `App\Models\User` means `App\App\Models\User`, and the error names a class nobody wrote. Here that
mistake cannot be written. The price is typing: reaching into a sub-namespace takes the full path or a
`use`, and a file wanting several names from one namespace writes one `use` per name.

**Declined:**
- **PHP's resolution kept as it is.** Two resolution rules and a token buy no meaning, and once a
  capability grant is keyed on a name's namespace (`rule:security/grants-are-keyed-on-a-namespace`),
  a name that resolves two ways is a grant that resolves two ways.
- **Relative qualified names kept, and only the leading separator removed.** It keeps the half where a
  name means different things in different namespaces, and removes the one way a reader could escape it.
- **A qualified name's first segment resolved through the imports.** It keeps the namespace-prefix import,
  at the cost of `App\Models\User` meaning one thing in a file that imports `App` and another in one that
  does not. `use Core\Json\Derive;` costs one line and reads as what it is.
- **The leading separator as the only way to write an absolute name.** It makes every `use` path and every
  `Core\Str` reference wrong, to reach a rule no easier to decide than this one.
