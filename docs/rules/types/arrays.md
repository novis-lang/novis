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
- **Invariant.** `array<int>` is not an `array<int|string>`; converting is `as array<int|string>` and
  costs an O(n) restamp (`rule:types/conversion`). What it does not cost is a copy — the two views
  share one copy-on-write buffer.
- The empty literal `[]` has type `array<never>`, which satisfies every `array<T>`.
- **Array literals are checked against the target type, never inferred and then compared.** Because
  every binding is annotated, a literal always has a target — which is why `var` refuses a bare one
  (`rule:types/var-inference`).
- At runtime an array header carries a pointer to an interned, immutable type descriptor: **one
  pointer per array header**, interned process-wide and O(distinct types in the program). Nesting is
  bounded at depth 32 with a diagnostic, so a pathological type cannot make checking superlinear.
- The stdlib's array signatures are parametric in `T`. Type variables belong to declarations the
  compiler owns; a call site may write a type argument only for a compiler-owned member that declares
  one it cannot infer. User-written generic functions and classes are not part of this.
