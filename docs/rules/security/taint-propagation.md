Any operation combining a tainted operand with an untainted one — concatenation, interpolation, a
string member, an array of scalars — produces a tainted result. This is the poisoned shape the type
system already uses on other axes, applied to a new one, and `secret` poisons independently beside it
(`rule:security/secret-propagation`).

**An encoder's result carries the text it encodes.** `Core\Json::encode` and `Core\Serialize::encode`
write every string of the value they walk into their result — JSON escaping leaves a `'` where it was,
and `decode` rebuilds every object of the graph — so the result is `tainted string` or `tainted bytes`
when the value carries outside text anywhere:
a tainted atom, or `mixed`, `object`, `iterable` or an unresolved type parameter, at any depth of an
array, a shape or an object's properties. An object is read through every class it can be at run
time, the declared class and each concrete class extending or implementing it. A value that provably
carries none — an `array<int>`, a shape or an object whose fields are all clean, an array literal of
the program's own values — encodes to the plain type. `Core\Uri::buildQuery` and
`Core\Uri::withQueryParameter` percent-encode every byte that is not safe in a URL, as `encodeFormValue`
does, and answer the plain type for that launderer's reason. `Core\Str::format` and `Core\Bytes::pack`
are encoders of their variadic arguments by the same rule: the template is a sink, and a `%s` filled
from outside text makes the result `tainted`.

**No `Core` signature loses a mark.** `crates/nvs-stdlib/tests/taint_signatures.rs` reads every row,
and fails naming each member that answers a plain `string` or `bytes`, or hands its callback one,
while a parameter could carry outside text the row does not classify: an array, `mixed`, `object` or a
type variable. Its allow list is the encoders above and the launderers
(`rule:security/launderers-are-sink-named`), each with the rule that makes it one.

**Text out of `mixed` is `tainted`.** `mixed` is where request input lands and the qualifier cannot be
written on it (`rule:security/tainted-qualifier`), so every way text leaves it adds the bit: `as string`
and `as ?string` answer `tainted string` and `?tainted string`, `as bytes` answers `tainted bytes`,
`as array<string>` answers `array<tainted string>`, `as secret string` answers `secret tainted string`,
and a `.` or an interpolation with a `mixed` operand is tainted. `$m is string` narrows `$m` to
`tainted string` on the true edge, and over a `mixed` subject `is tainted {…}` and `is tainted string`
are admitted. The same holds for an `array<mixed>` operand and for a union with a `mixed` member.
One exception: a union's `array<mixed>` arm adds nothing to an `as` whose target holds no array,
because `as` never turns an array into a scalar (`rule:types/conversion`). So
`Core\Config::setting`'s `?(string|int|float|bool|array<mixed>)` converts `as string` to a plain
`string`, and `as array<string>` still answers `array<tainted string>`.
So a member that answers `mixed` or `array<mixed>` accepts `tainted` text where its parameter is
contagious: `Core\Json::decode` of a `tainted string` answers `mixed`, and the mark is on each
string read out of it.
**The keys of such a value are outside text too**: a `foreach` key over a `mixed` or `iterable`
subject, or over an array whose element type carries `tainted` or `mixed` anywhere — `queryArray`,
`postArray`, `headers()`, a decoded `array<mixed>` — is `tainted string`, because an array's type says
nothing of its keys and its elements are the evidence of where it was filled. A `Core\Arr` member that
returns a key, or hands one to its callback, follows the same test: `keys`, `firstKey`, `lastKey`,
`keyOf` and `findKey` over such an array answer `tainted string`, and so does the `$key` of a `map`,
`filter` or `reduce` callback. Every member that fills
an array from outside answers one with such elements, so the element type covers members not yet
written too. An array the program fills with tainted values under its own literal keys over-taints,
which is the safe direction: write `tainted string $k`, and bind or escape the key where it reaches a
sink. The bound is a map nested in a decoded target: a `postAs<Form>` whose `Form` has an `array<int>`
field is keyed by the peer and has clean elements, so its keys bind as plain `string`. An object's fields are
read the same way: a shape target of `as` or `is` over a class takes each text field from the field of
that name, so a field that is `mixed`, `object` or tainted, or one the class does not declare and only
a subclass could, counts as outside text. Where the text would land in a type the program *wrote* — a
shape target of `as` or `is` over a `mixed` or `object` operand or over such an object, or a `foreach`
key or value binding — the qualifier is never added behind the declaration: a field or binding written
without `tainted` is a diagnostic asking for `tainted {…}` or `tainted string` (`E0851`), and a `var`
key takes `tainted string` itself. `object` counts with `mixed` for a shape target because its fields
are as untyped as `mixed`'s, so `$m as object` is no step round the rule. A shape target whose
qualified fields are all `tainted` is admitted by `as`, and over an unchecked subject by `is`, and is
tested with the qualifier erased; a `secret` field stays refused. A literal that was stored in `mixed`
and is genuinely trusted, or the text of a `Stringable` object pulled out of one, is recovered with
`rule:security/assert-trusted`, or with the sink's own launderer.

**A key from outside enters only an array whose keys read back tainted.** Since the element type is
the only evidence, `$m[$key] = …` with a `tainted` key into an array whose element type carries
neither `tainted` nor `mixed` is `E0401` at the key. So are `??=`, every compound write, a nested
level such as `$m[$key]["n"] = …`, and a literal's `[$key => …]` against its declared or inferred type.
`unset` adds no key and is not checked. The cost is an update to a key already there, such as a count
per status read from a row: the checker cannot see that the key exists, so the program compares the
text with its own keys and writes through the matching one, or passes the key through
`rule:security/assert-trusted`.

The rule sits at the one boundary every `mixed` source shares — `Core\Request::json`,
`Core\Script::args`, `Core\Session::get`, `Core\Json::decode`, `Core\Cache::get`, a row's `get`,
`Core\Serialize::decode`, `Core\Uri::queryParameter`, a socket's or event stream's `value` — so it
covers members not yet written as well.

A checked `as` conversion to a type that already throws on a malformed shape — `as uint`, `as int`,
`as float`, `as bool`, an enum's backing type, a set of allowed values — **removes the qualifier on
success**, from a tainted operand and from a `mixed` one alike. No new syntax is needed: a value that
survived the check has had its shape proven, which is what laundering means for a non-string type.
`as ?T` decides the qualifier by exactly this rule and launders nothing of its own
(`rule:expressions/conversion-keeps-qualifiers`).

`bytes as string` and `string as bytes` **preserve** the qualifier in either direction, and so does a
conversion out of a `?tainted string` or an `array<tainted string>`. UTF-8 validity says nothing about
whether the content is safe for a given sink, and a conversion that laundered here would be a one-word
bypass of every rule below. All of it is decided while checking: no tag at run time, nothing on the
request path.

- **Memory:** none. The qualifier is erased before codegen; the cost is compile time, one walk of the
  operand's type per conversion, `.`, interpolation and type test, one walk of the subject's element
  type per `foreach` with a key, and one property lookup per text field of a shape target over an
  object.

**Declined:**
- **Typed request accessors as the only fix.** They leave `json()`, `Core\Script::args`,
  `Core\Session::get`, `Core\Cache::get` and a row's `get` open, and every later member answering
  `mixed` would reopen the gap.
- **A hidden, source-marked `mixed`** carrying an invisible "came from outside" bit. It is lost at the
  first `mixed` parameter or property, it cannot be written in a declaration, and `tainted mixed` is
  refused for the same reason (`rule:security/tainted-qualifier`).
- **A key qualifier on the array type**, such as `array<tainted string => T>`, set by the members
  that fill an array from outside. It is exact, and it closes the nested-map bound above, but it adds
  a type parameter to every array and a second axis to assignability, which simplicity does not buy
  until that shape turns up in practice. A write with a `tainted` key into a plain-keyed array is
  refused instead, which keeps the element type's evidence true.
- **Letting a `tainted` key through when it updates a key that is already there.** Whether it is
  there is a run-time fact, so the checker would have to trust an `Core\Arr::hasKey` test that the
  next statement can invalidate.
- **Tainting a written shape target or binding without `tainted` written.** The declaration would then
  understate the value, and a type that is not what it says is worse than a long one.
- **Refusing `as string` out of `mixed` outright.** It leaves no way to read text from a request at
  all, and the tainted result is exactly what a sink needs to see.
