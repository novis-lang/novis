`[errors] deprecated` decides what a use of deprecated code does while it runs: `"ignore"`, `"log"` or
`"throw"`, and `"ignore"` by default in every mode.

```toml
[errors]
deprecated = "throw"   # "ignore" (default) | "log" | "throw"
```

Deprecated code still works in production, and only a developer turns this on. The directive is
`Runtime`-class and reloadable, so a test or a single request may set it with `Core\Config::set`, for
that request alone. `[mode]` does not change it.

- **`"log"`** writes one `warning` record per use site per request, naming the member, the use's file
  and line, and the replacement. The request remembers the sites it has logged in a set freed with it.
- **`"throw"`** throws `Core\DeprecatedError`, a `LogicError`, whose message is `W1003`'s text
  (`rule:attributes/a-deprecation-names-its-replacement-as-code`), fixed at compile time and stored once
  in the unit's constant pool.

**Where the check runs.** A deprecated method or constructor checks on entry, so a call through an
interface or a dynamic call is caught. A read or write of a deprecated property, class constant or enum
case, a `new` of a deprecated class, and a passed deprecated parameter check at the use. A type position
checks nothing, because it runs nothing. The check is a load of one word in the request context and a
branch to a cold helper when it is not zero; a program that uses no deprecated code emits no check. A
use of deprecated code pays that load and branch every time it runs, whatever the setting.

- **Memory:** under `"log"`, the set of sites a request has logged, per request, O(distinct deprecated
  sites the request reaches), freed with the request. Under `"ignore"` and `"throw"`, nothing per
  request.
