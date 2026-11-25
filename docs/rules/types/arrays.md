The container is an insertion-ordered hash with copy-on-write value semantics.

- **A write copies a level only when another holder shares it, at every depth.** `$g[$i][$j] = $v`
  writes an unshared row in place and copies a row or an array that a second variable, a `foreach`
  or a closure also holds. The decision is the reference count at run time, so a nested write into an
  unshared array costs what it adds, not what the row holds. A write that throws changes no level.
  `.=` on a `string` element, a plain `string` property or a `string` static property grows the
  string in place by the same count, and an append to an element builds no row: an absent key throws
  as a read does. `.=` of an interpolation or a `.` chain onto a local, a plain property or a static
  property writes each part into the target and builds no joined string. A part that is the target
  itself makes the append copy. A statement that replaces a value with one made from it, such as
  `$x = f($x)`, changes it in place wherever no other holder can see the old one, as
  `rule:types/a-replaced-value-is-changed-in-place` says.
- **A typed check of the elements takes one step when the array's tags fit.** Every array keeps
  the union of the tags of every value ever written into it, and a removal never lowers it. A check
  that an argument, a property or a callback's parameter is `array<int>`, `array<?string>` or any
  element type a tag alone decides reads that word, and walks the entries only when it names a tag
  the type does not admit. So a `reduce` whose carry is an `array<uint>` costs each step its own
  work, not the carry's length. A write through a nested row's slot makes the word unknown, and the
  check of that array walks. **What it spends:** one word per array and one OR per write.

**Every key is a `string`.** There is no integer key. One key type means one key domain: which
subscripts name the same entry is decided once, at the subscript, and nothing later converts a key
between two kinds.

- `$a[] = $v` appends under the next integer index rendered in decimal — `"0"`, `"1"`, `"2"` — from
  a counter the array keeps. A `Core\Arr` member whose result keeps the subject's keys keeps that
  counter too, as `unset` does, whether the subject was shared or not. `Core\Arr::append` appends
  from that counter, as `$a[] = $v` does. A list cut only at its end appends at its new length.
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
- What comes back from an array keyed by strings is a `string` everywhere: `Core\Arr::keys()` returns `array<string>`, so do
  `keyOf`, `firstKey`, `lastKey`, `findKey` and `flip`, and so is the `$key` a callback is handed —
  on a packed list exactly as on a map. A key **parameter** still takes `int|string`, because the
  subscript normalisation above is what makes those one key. A key binding declaring any other type is
  refused where it is written (`E0723`), because there is no array that could fill it. A callback
  that declares `int $k` throws `LogicError` at the call, on a list exactly as on a map.
- **Memory:** a packed list keeps a key as a position, so a `Core\Arr` member renders one decimal
  string per entry for its callback, and only where the callback declares a key parameter.
- `Core\Json::encode` writes a JSON array exactly when the keys are `"0"` … `"n−1"` in order, and a
  JSON object otherwise, so string keys change nothing in the encoded output.

**An array may be keyed by the cases of an enum: `array<E, T>`.** The key parameter comes first and
is optional, and it must be an enum, file-scope or class-scoped. Any other key type, `array<string, T>`
too, is `E1017` where it is written, and its help says that `array<T>` is already keyed by strings.
The type is `Ty::Array(elem, key)`, with `key` the enum or nothing, so a reader that ignores the key
reads the `array<T>` the value is assignable to, and every qualifier walk still sees the element.

- **Storage does not change.** A key is stored as any `int` subscript is: a position in a packed list,
  a decimal string otherwise. The array keeps no enum identity at run time.
- **A write and a read are held to `E`.** The key of `$a[$k] = $v` and of `$a[$k]` must have the
  static type `E`. An `int`, a `string`, a case of another enum and a `mixed` key are `E0434`, naming
  `E`, and the help names `as E`. `$a[] = $v` is `E1018`: there is no next case to append under.
  An absent key throws on a read, as it does on any array.
- **A literal is checked against `array<E, T>` where one is its target.** With no target, a case key
  is still `E0434`, and its help names `array<E, T>`. A literal is never inferred to be enum-keyed.
- **A key read back is a case.** A `foreach` key over `array<E, T>` has the type `E`, and `E0723`
  takes a key declared `E` there. `Core\Arr::keys`, `firstKey`, `lastKey`, `keyOf` and `findKey`
  return `E` or `array<E>`, and a callback's key parameter may be `E`. The stored key becomes a case
  with no check, because every write was checked.
- **`array<E, T>` is read wherever `array<T>` is expected**, with its keys as their decimal strings.
  The element stays covariant and the key is invariant, for the copy-on-write reason below. The other
  direction is only `as array<E, T>`, which checks every key: a list's key is its position, and a
  map's key must be the decimal of a case exactly, so `"08"` is not the case `8`. A miss throws
  `RuntimeError`, or gives `null` under `as ?array<E, T>`.
- **A `Core\Arr` member whose result keeps the subject's keys returns `array<E, U>`.** The members are
  the roster `nvs_stdlib::arr::KEEPS_KEYS`, and the members that give keys back are
  `nvs_stdlib::arr::ENUM_KEYS`. `slice`, `reverse` and `sort` keep keys only under `preserveKeys`, so
  their result is `array<T>`, and `as array<E, T>` converts it back. `groupBy` with a callback that
  returns `E`, and `flip` of an `array<E>`, return an array keyed by `E` (`nvs_stdlib::arr::CASE_KEYS`).
  `flip` of an `array<E, string>` returns `array<E>`. `groupCounts` stays keyed by strings.
- `Core\Json::encode` writes a case key as its number. A decoded object, a cached array and a session
  value become enum-keyed with `as array<E, T>`.
- **Memory:** nothing per array and nothing per entry. A key read back from a map parses a short
  decimal string with no allocation, and a packed position costs nothing. `as array<E, T>` costs one
  check per key.

**The element type may be declared, and nests to any depth** — `array<uint>`, `array<array<int|string>>`,
`array<User|null>`. The key is a `string`, or an enum given as a first parameter.

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

An array has no internal cursor. A cursor inside a copy-on-write value would be copied with the array,
so a copy would carry its iteration position. `foreach` and `Core\Arr`'s `first`, `last`, `firstKey` and
`lastKey` cover every use.

**A list has room at both ends, and a map knows where its live entries start.** Dropping a list's
first entries releases them and moves its front, and a prepend writes into the room in front and grows
by doubling where there is none. A map keeps the slot of its first live entry, so `first`, `firstKey`
and the start of a `foreach` take one step past any number of removed entries. `last` and `lastKey`
take one step on a list, and on a map walk back past the holes at its end, which compaction bounds.
The room and the holes are freed when they outnumber the live entries. **Memory:** a word per list and
per map. A list that was prepended to holds up to twice its entries, and one cut down a step at a time
up to four times; a list never prepended to or cut holds nothing more.

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
- **`array<int, T>`, or any key type but an enum.** It brings back the integer key domain the first
  item above declines.
- **A case as a key of a plain `array<T>`.** It would key by a number another enum shares, and give
  the key back as a `string`. `E0434` stays there.
- **A second array variant, `Ty::EnumArray`, for the keyed form.** Every reader that is not exhaustive
  over `Ty` would miss it, and a qualifier walk among them would miss a `secret` or `tainted` element.
- **The case name as the JSON key.** The array keeps no enum identity at run time. Writing names would
  cost a tag per array, and decoding would need a second mapping.
- **A literal that must name every case.** The covering `match` is the total form
  (`rule:expressions/a-match-over-an-enum-covers-every-case`), and a counting array starts empty.
- **A `Core\Arr` signature variant for a kept key.** About forty walks match `CoreTy::Array` without
  being exhaustive, the taint walks among them, so the rosters above name the members instead.
- **`groupCounts` keyed by `E`.** Its `by` callback is in an options bag, and a bag binds no type
  variable. `as array<E, uint>` converts its result.
