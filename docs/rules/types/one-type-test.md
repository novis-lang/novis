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
Novis design question, decided on Novis's priorities. What PHP does with its own pattern-matching
proposal or with `instanceof` is not a reason to revisit this rule.

**Two operators would split one question, and the seam is where the cost is.** A class reference and a
written class name would be tested with different words; a subject that can hold no object would be
refused by one and answered `false` by the other, so rewriting one into the other would change
answers; and every rule touching either would explain why the other exists. Diverging from PHP here
costs one mechanical rewrite, `$x instanceof C` to `$x is C`, and a PHP program's `instanceof` never
compiles to something else.

**Memory:** none. `is C`, `is $cls`, `is callable`, `is iterable` and a `catch` chain all emit the
same class-test instruction and the same descriptor walk.

**Declined:**
- **`Core\Reflect::isInstance($x, $cls)` for the dynamic test.** It asks the question `$x is Request`
  asks in another vocabulary, cannot narrow, because the checker does not read a library call's result
  as a type, and leaves two ways to ask.
- **`instanceof` kept for the dynamic form alone.** It keeps a keyword and its whole path through the
  parser, the checker, codegen and the language server alive for one form, and keeps two spellings
  whose answers differ for a subject that can hold no object.
- **Waiting for PHP to settle pattern matching.** It would shape the surface around an undecided
  proposal, a cost paid by every later change, where diverging is paid once.
