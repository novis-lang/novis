A PHP built-in that has no successor here is absent for one of four standing reasons:

1. **A pure alias** — `sizeof`, `join`, `chop`, `key_exists`, `pos`, `fputs`, `is_integer`, `doubleval`.
2. **Dead or dying in PHP itself** — `ereg*`, `mysql_*`, `mcrypt`, `create_function`, `each`,
   `money_format`, `utf8_encode`, `strptime`, `get_browser`.
3. **Already closed by another rule** — about 120 functions whose removal follows from declared types, from
   anonymous functions and method references being the only callables, from the closed doors, from the escalation ladder
   (`rule:errors/escalation-ladder`), from having no superglobals
   (`rule:statements/no-host-populated-variables`), or from having no cross-request ambient state.
4. **Structurally wrong here** — the internal array pointer, because a mutable cursor inside a
   copy-on-write value is incoherent; every by-reference mutator (`rule:core-api/nothing-mutates`);
   `array_merge` and the `+` operator over two arrays, whose rule is chosen by a key's *type* in a language
   with one key type (`rule:types/array-combination`); the half-escapers, whose false confidence taint
   tracking exists to prevent; and `settype`/`gettype`/`strval` (`rule:types/no-legacy-cast`).

No list maps a PHP name to its outcome (`rule:programs/no-compatibility-promise`): the four reasons are
how a request for a missing built-in is judged, and the `Core` library states what Novis has.
