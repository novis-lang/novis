Novis has one type test, `$x is T`, and `instanceof` is refused where it is written (`E0253`): the
diagnostic names the rewrite — *write `$x is Request`; a class reference on the right is `$x is
$cls`*. `is` asks every question a second class-test operator would, so a language with both would be
carrying a keyword for a special case of its own operator. The refused word does not compile, so a
program never changes meaning silently over it.

`rule:types/type-test` is the operator and owns its table. This rule owns the refusal and the value
arm.

| Written | Answer | Why |
|---|---|---|
| `$x instanceof C` | `E0253`, help *`$x is C`* | one type test |
| `$x is $name` where `$name` is a `string` | `E0496`, help *`as class<T>`* | a class reference is checked where it is made, not at the test (`rule:types/class-reference-sites`) |

**`$x is $cls`** tests the class a value holds against the descriptor a `class<T>` carries, and
narrows its subject to `T` on the true edge (`rule:types/narrowing`). The value arm starts with `$`;
every other token after `is` starts a type.

The shapes a pattern grammar would need — object and array patterns, comparison patterns, pinning,
`match ($x) is {…}` — keep refusing as syntax Novis does not have. Any future pattern syntax is a
Novis design question, opened by its own record and decided on Novis's priorities.
