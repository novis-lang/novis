The container is an insertion-ordered hash with copy-on-write value semantics.

**Every key is a `string`.** There is no integer key. One key type means one key domain: which
subscripts name the same entry is decided once, at the subscript, and nothing later converts a key
between two kinds.

- `$a[] = $v` appends under the next integer index rendered in decimal — `"0"`, `"1"`, `"2"` — from
  a counter the array keeps.
- An `int` or `uint` subscript is normalised to its decimal string at the subscript: `$a[8]` is
  `$a["8"]`. That is key normalisation, not a conversion, and needs no `as`. `"08"` stays a distinct
  key from `"8"`.
- A `...$a` spread in an array literal, and one filling a variadic tail, renumber an integer-looking
  key and preserve every other — each entry copied is either the append above or the write
  `$a[$k] = $v`. A spread therefore throws exactly where an append throws.
- A `float`, `bool` or `null` subscript is **rejected**: none of them is a key, and nothing converts
  one into a key. Truncating `1.7` to `"1"` or reading `null` as `""` would be a silent conversion at
  the one place where a mistake becomes a missing row.
- Iteration order is insertion order, always; only the sort members reorder, and they say so in their
  names. Binary `+` and `+=` over two arrays are a diagnostic naming `Core\Arr::underlay`
  (`rule:types/array-combination`).
- What comes back is a `string` everywhere: `Core\Arr::keys()` returns `array<string>`, so do
  `keyOf`, `firstKey`, `lastKey`, `findKey` and `flip`, and so is the `$key` a callback is handed —
  on a packed list exactly as on a map. A key **parameter** still takes `int|string`, because the
  subscript normalisation above is what makes those one key. A key binding declaring any other type is
  refused where it is written (`E0723`), because there is no array that could fill it. A callback
  that declares `int $k` throws `LogicError` at the call, on a list exactly as on a map.
- **Memory:** a packed list keeps a key as a position, so a `Core\Arr` member renders one decimal
  string per entry for its callback, and only where the callback declares a key parameter.
- `Core\Json::encode` writes a JSON array exactly when the keys are `"0"` … `"n−1"` in order, and a
  JSON object otherwise, so string keys change nothing in the encoded output.

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
  `array<int|string>` takes an `array<string>`. It spends nothing: the call passes the array it was
  handed, and nothing is allocated, copied or interned for it.
- **The `int` → `float` widening is not part of the covariance.** An element of an array that already
  exists keeps the representation it was stored with, so an `array<int>` or `array<uint>` is not an
  `array<float>`, at any depth and in any position (`rule:types/implicit-widening`). `array<int|float>`
  still takes it, because `int` is one of its members. Passing, assigning or returning one there is
  `E0401` (`E0403` for a return), and the help line names the conversion, `$value as array<float>`.
- **The read is free; the conversion is not.** `as array<int|string>` is still the spelling that
  *restamps*, at O(n), one tag test per element (`rule:types/conversion`) — it is what converts an
  array, where the covariant read only passes one along. The two views share one copy-on-write
  buffer. `as array<float>` over `int` or `uint` elements is the one conversion that copies: it builds
  a new array of the same size with each such element converted, at each level that has such
  elements. The keys and their order are kept, an element above 2^53 throws, and `as ?array<float>`
  gives `null` there.
- **Memory:** that copy, once per conversion, freed with the array. A literal at `array<float>` and
  every read of an `array<float>` cost nothing extra.
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
  one it cannot infer, such as a decode handed a `string` with nothing else to say what it becomes.
  That door is a roster the compiler holds, `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS` —
  `Core\Arr::shapeAs<T>`, `Core\Json::decodeAs<T>`, `Core\Request::jsonAs<T>` and `Core\Db`'s
  `queryAs<T>` among them — so a member gaining the door is a row in that table and the conformance
  case the coverage gate asks for, not a change to this rule. What may be written there is a class, an
  inline shape (`rule:types/shape-type`), or an `array<…>` of either where the source is a list;
  anything else is refused where it is written (`E0465`). Nesting stops one level down:
  `array<array<C>>` is not a document shape any format gives a field. A class and a shape differ only
  at the call site: a class raises two questions about the rest of the program — whether it carries the
  deriving attribute, and whether its properties fit the format — which is why a decode site is
  recorded against it, while a shape declares its own fields where it is written, and its contract is
  read off the type and emitted beside the call. User-written generic functions and classes are not
  part of this (`rule:types/no-user-generics`).
- **Memory:** a shape at the type-argument door is compile-time work and one interned descriptor,
  O(distinct types in the program) and nothing per request.

**Declined:**
- **Integer keys kept beside string keys.** Two key domains need a rule for converting between them,
  and that rule is where `$a[8]`, `$a["8"]` and `$a["08"]` stop meaning what they look like.
- **A callback's `$key` handed on as a position on a packed list.** The key's type would then depend
  on how the array happens to be stored, which is the integer-key and string-key split again.
- **A type-argument door that admits a class alone.** A shaped decode would then need a class declared
  only to be named once, and a request boundary would be the one place a program must invent a nominal
  type to describe data it does not own.
- **`array<T>` invariant, with the union annotated at the caller.** A binding that only ever holds
  strings would be declared `array<int|string>` to satisfy a callee, which says something false about
  the value and puts the callee's type into the caller's code.
- **Covariance with a runtime barrier on writes through a widened view.** That is what a language with
  aliasing arrays needs, and here it would spend request-path time on a hazard copy-on-write values
  already exclude.
- **Converting an `array<int>` to `array<float>` at an assignment, argument or return.** It hides
  an O(n) copy inside a statement where nothing in the source shows it.
- **Converting an `int` element when an `array<float>` is read.** It costs a tag test on every read
  of a float array, the most common array read on the request path, and leaves `int` tags in an
  array declared to hold floats, for `Core\Json::encode` and `Core\Arr::sum` to see.
- **The roster listed in prose as the authority.** A list in a document cannot track a table, and the
  door's members are the compiler's registry.
