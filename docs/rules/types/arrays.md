The container is PHP's insertion-ordered hash with copy-on-write value semantics, unchanged. Two
things change.

**Every key is a `string`.** There is no integer key.

- `$a[] = $v` appends under the next integer index rendered in decimal — `"0"`, `"1"`, `"2"` — from
  the counter PHP already keeps, so lists behave as they always did.
- An `int` or `uint` subscript is normalised to its decimal string at the subscript: `$a[8]` is
  `$a["8"]`. That is key normalisation, not a conversion, and needs no `as`. `"08"` stays a distinct
  key from `"8"`, exactly as in PHP.
- A `...$a` spread in an array literal, and one filling a variadic tail, renumber an integer-looking
  key and preserve every other — each entry copied is either the append above or the write
  `$a[$k] = $v`. A spread therefore throws exactly where an append throws.
- A `float`, `bool` or `null` subscript is **rejected**, where PHP truncates, stringifies `true` to
  `"1"` and `null` to `""`.
- Iteration order is insertion order, always; only the sort members reorder, and they say so in their
  names. Binary `+` and `+=` over two arrays are a diagnostic naming `Core\Arr::underlay`
  (`rule:types/array-combination`).
- What comes back is a `string` everywhere: `Core\Arr::keys()` returns `array<string>`, so do
  `keyOf`, `firstKey`, `lastKey`, `findKey` and `flip`, and so is the `$key` a callback is handed —
  on a packed list exactly as on a map. A key **parameter** still takes `int|string`, because the
  subscript normalisation above is what makes those one key. A key binding declaring any other type is
  refused where it is written (`E0723`), because there is no array that could fill it.

**The element type may be declared, and nests to any depth** — `array<uint>`, `array<array<int|string>>`,
`array<User|null>`; one parameter, not two, because the key type is fixed.

- **Enforced on every write** — `$a['k'] = $v`, `$a[] = $v`, `+=`, and every `Core\Arr` member that
  builds an array. Where the value's static type satisfies the element type the check is compile-time
  and free; through `mixed` it is a runtime check, and a failure is a throw like any other
  (`rule:errors/propagation`).
- **Covariant in its element type, and the only generic name in the language that is.** An
  `array<int>` satisfies an `array<int|string>` wherever one is *read* — a parameter, a return, an
  assignment to a wider binding — and never the reverse, a narrowing still being written
  `as array<int>`. The usual objection does not reach it: covariance is unsound where the wider view
  *aliases* the narrower one, because a write through the wide view lands in storage the narrow one
  still reads, and an array here is a copy-on-write **value** instead. The widened binding is a
  separate array the moment anything writes to it, so the narrow one can never observe the write, and
  a write through the wide view is checked against the wide view's own element type. What the
  covariance buys is every signature written over a union — `Core\Arr::sum`'s
  `array<int|float|decimal>` takes the `array<int>` a caller means by it, and `Core\Arr::flip`'s
  `array<int|string>` takes an `array<string>`.
- **The `int` → `float` widening is not part of the covariance.** An element of an array that already
  exists keeps the representation it was stored with, so an `array<int>` or `array<uint>` is not an
  `array<float>`, at any depth and in any position (`rule:types/implicit-widening`). `array<int|float>`
  still takes it, because `int` is one of its members.
- **The read is free; the conversion is not.** `as array<int|string>` is still the spelling that
  *restamps*, at O(n), one tag test per element (`rule:types/conversion`) — it is what converts an
  array, where the covariant read only passes one along. The two views share one copy-on-write
  buffer. `as array<float>` over `int` or `uint` elements is the one conversion that copies: it builds
  a new array of the same size with each such element converted.
- **An array literal is checked against its target type wherever one is written, never inferred and
  then compared.** Each element is placed at the element type, so `[1, $count]` at `array<float>`
  stores two floats, at any depth and in a shape field, and `[]` is an `array<T>` at every `array<T>`.
  A ternary's arms, the default after `?:` and the right operand of `??` have the target of the whole
  expression. Where that names none, a literal after `??` or `?:` is placed at the left operand's type
  without `null`: `$values ?? []` over `?array<float>` is an `array<float>`, and `$names ?? [1]` over
  `?array<string>` is a mismatch at the `1`. Under `var` a literal has no target, and is
  `array<T>` only when every element has the one type `T`; any other literal is refused there
  (`rule:types/var-inference`). A literal in any other position with no target, such as an `echo`
  argument, is `array<mixed>`.
- At runtime an array header carries a pointer to an interned, immutable type descriptor **exactly
  where something reads one back**: **one pointer per array header**, interned process-wide and
  O(distinct types in the program). An array every write to which was checked as it was compiled has
  nothing to read it for, and carries none — the checks above are against the *target* type at the
  write and at the `as`, never against a type the array remembers. Nesting is bounded at depth 32
  with a diagnostic, so a pathological type cannot make checking superlinear.
- The stdlib's array signatures are parametric in `T`. Type variables belong to declarations the
  compiler owns; a call site may write a type argument only for a compiler-owned member that declares
  one it cannot infer, and that door is a roster of four — `Core\Arr::shapeAs<T>`,
  `Core\Json::decodeAs<T>`, `Core\Request::jsonAs<T>` and `Core\Db`'s `queryAs<T>`. What may be
  written there is a class, an
  inline shape (`rule:types/shape-type`), or an `array<…>` of either where the source is a list;
  anything else is refused where it is written. User-written generic functions and classes are not
  part of this.
