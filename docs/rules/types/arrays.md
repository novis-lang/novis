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
- **The read is free; the conversion is not.** `as array<int|string>` is still the spelling that
  *restamps*, at O(n), one tag test per element (`rule:types/conversion`) — it is what converts an
  array, where the covariant read only passes one along. Neither costs a copy: the two views share
  one copy-on-write buffer.
- The empty literal `[]` has type `array<never>`, which satisfies every `array<T>`.
- **Array literals are checked against the target type, never inferred and then compared.** Because
  every binding is annotated, a literal always has a target — which is why `var` refuses a bare one
  (`rule:types/var-inference`).
- At runtime an array header carries a pointer to an interned, immutable type descriptor: **one
  pointer per array header**, interned process-wide and O(distinct types in the program). Nesting is
  bounded at depth 32 with a diagnostic, so a pathological type cannot make checking superlinear.
- The stdlib's array signatures are parametric in `T`. Type variables belong to declarations the
  compiler owns; a call site may write a type argument only for a compiler-owned member that declares
  one it cannot infer, and that door is a roster of four — `Core\Arr::shapeAs<T>`,
  `Core\Json::decodeAs<T>`, `Core\Request::jsonAs<T>` and `Core\Db`'s `queryAs<T>`. What may be
  written there is a class, an
  inline shape (`rule:types/shape-type`), or an `array<…>` of either where the source is a list;
  anything else is refused where it is written. User-written generic functions and classes are not
  part of this.
