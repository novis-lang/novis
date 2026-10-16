`var` stands in for a written type in two places — a local declaration and a `foreach` binding — and
the binding takes its type from the expression it is filled from, then keeps it for good. In
`var $name = expr;` that expression is `expr`. In `foreach ($subject as var $k => var $v)` it is the
subject: `$v` takes the subject's element type, and `$k` takes `string`, the one type an array's key
binding may have (`rule:types/arrays`).

The type is computed exactly as it is for any position with no expected type, and it is then fixed on
the binding like a written one — `rule:types/declaration`'s "no binding's type ever changes" is
untouched. It is exact and honest: `var $n = 1;` gives `int`, `var $id = Core\Request::query('id');`
gives `?tainted string`, `var $doc = Core\Request::json();` gives `mixed`, and a `var` value binding
over a `mixed` or `iterable` subject gives `mixed`, because that is what the source expression is. A
qualifier, such as `tainted`, arrives on the binding with the type it qualifies. A `foreach` binding's
`var` passes through the same `void` and `never` refusal a local's does. `inout var $v` binds at the
element type itself, so its two sides declare the same type by construction.

A local's initializer is mandatory: `var $n;` is a parse error naming `=`. A `foreach` binding with
neither a type nor `var` is the parse error it always was.

A **bare array literal** is `array<T>` when every element has the same type `T`, and is refused
otherwise. An element's type is what `var` gives that element alone, so `var $ids = [1, 2];` is
`array<int>`, `["a" => 1]` is `array<int>` because keys take no part, a nested literal is inferred the
same way first, and a spread gives its source's element type. Nothing is widened to a common type:
`[1, 2.5]`, `[$user, $admin]` with `Admin` a child of `User`, and `[$tainted, "plain"]` are each two
types. Those and an empty literal at any depth are `E_VAR_ARRAY_LITERAL_NEEDS_TYPE`, whose help names the declaration to write: the union of the
element types, `?T` where the other one is `null`, or `array<T>` for an empty literal. An element that
reported its own error, a spread over something that is not an array (`E0484`) included, adds no
second diagnostic, and the literal is then not inferred. An inferred literal records each element at
`T` exactly as a written `array<T>` target does, so the two declarations lower the same. A bare literal
as the subject of a `var` value binding follows the same rule, and the help there is a typed local,
iterated. A `var` key over a literal is never refused, since its type comes from the array's key and
not from the elements. The rule is on the expression's own top level only — in `var $x = f([1, 2]);` and
`foreach (f([1, 2]) as var $x)`, `f`'s parameter is the literal's target.

Every other rule that applies to a typed binding — declare-once, definite assignment, redeclaration
diagnostics, the cursor's missing key — applies unchanged, because by the time those checks run `var`
has already resolved to a concrete type. A later write that does not fit that type is the mismatch a
written type gets, with one addition, because the type it misses is not on the line: a label on the
`var` line, and a help naming the declaration that would accept the value — `var $prices = [10, 20];`
then `$prices[] = 12.5;` names `array<int|float> $prices = [10, 20];`, the element type widened at
the depth the write went through.

`var` is not a solver. It reads only the filling expression, never how the binding is used later, and
it finishes in one pass with no backtracking, so there is no second, inferred type system to keep in
agreement with the declared one. Nothing after the checker sees it: every binding lowering and codegen
meet already has a concrete type, exactly the one a written annotation would have given. `var` starts
nothing else at statement position, so the parser needs no trial parse to tell it apart. What it costs
is legibility: the type is not on the declaration line, and finding it means reading the initializer.
An editor's inlay hint shows it; a diff shows nothing. In a `foreach` header that is the price of not
writing twice the type the subject already states.

- **Memory:** none. `var` is resolved entirely while checking, before any IR exists.

**Declined:**
- **Inference across a binding's later uses**, widening or narrowing the type from later assignments.
  That is the second type system `rule:types/declaration` rules out; a later write is checked against
  the type the initializer gave.
- **`var $x = [];` typed as `array<never>`.** The binding could never usefully hold anything without a
  further `as`, which is a worse trap than a diagnostic at the declaration.
- **A new keyword instead of `var`.** `var` was already a reserved token with no meaning in Novis, so
  reusing it needed no lexer change and collides with nothing.
- **A common supertype for a literal's elements.** It would give `[$user, $admin]` the type
  `array<User>`, a choice the author did not make. One type, or the author writes it.
- **Inferring the union of a literal's element types.** A union attached to a binding unseen is a type
  nobody chose (`rule:ide/no-compile-path-calls-the-synthesis`); the help line prints it to be written.
- **A new code per refused literal, or for a bare literal under a `var` value binding.** Every refused
  literal is the one mistake `var` cannot type, and one mistake gets one code; the message names the
  case.
- **`var` as a house style.** A written element type tells a beginner what the loop or the array has,
  so a program keeps its written type and neither form is preferred in a new one.
- **An untyped `foreach ($xs as $v)` meaning `var`.** The binding would look like PHP's and mean
  something PHP does not; `var` says in the source that the type was taken.
