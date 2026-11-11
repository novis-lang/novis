Fields are every instance property in the class chain, the root parent's first, each class's in
declaration order, private ones included. A parent counts whether it carries the derive itself or
not, and its own `Field` attribute renames or skips the field for every subclass. A property a
subclass declares again is one slot, so it is one field: at its topmost declaration's position, with
the nearest declaration's type and `Field` attribute. A parent with no source, such as a `Core`
class, gives no fields. Visibility is an access-control decision and has no business deciding a wire
format; declaration order fixes encode order, so output is byte-deterministic across runs and
machines. Every rule below holds for an inherited field as for an own one, against the subclass's
constructor, and a key is unique across the whole chain. A refusal of an inherited field is reported
at the subclass's attribute, with a second label on the parent's property, because the subclass is
the class being derived.

**Every non-skipped field must also be a constructor parameter of the same name and type**, or the
derive is a compile error at the attribute. For a class written with promoted parameters the two
lists are literally the same declaration, so this costs nothing. It is worth its cost because a
decode is an ordinary `new`: the generated decoder fills locals and calls the constructor, so a
decoded object is indistinguishable from a hand-built one and every invariant the constructor
establishes still holds. A deriving class with no written `constructor` is therefore one compile
error at the attribute (`E0460`), even when every property carries a default: with no parameter
list, no field has a position to arrive through. A `lateinit` property cannot be a field, being by definition not
constructor-assigned; `readonly` is unaffected, because the constructor assigns it like any other.
Because the two lists are checked to agree at compile time, `fromJson(toJson($x))` rebuilding `$x`
is a property of the mechanism rather than of the care taken writing two functions, and a renamed
property breaks loudly: every reader of the old key gets a missing-field issue with its path.

A field's type must be codec-reachable — a scalar, `mixed`, one of the named `Core` value types, an
enum, a set of allowed values, an inline shape, an `array<T>` or `?T` of one of those, or another
class that itself has a codec. Anything else is a compile error at the field (`E0756`), whose help
names `mixed` and a set of allowed values. A set of allowed values is `rule:types/single-value-types`'s
union whose every non-`null` member is a single-value `string`, or every one a single-value `int`,
one value included; `"a"|"b"|null` and `?"a"` are nullable as any `?T` is. It is its base type on
the wire. Encode writes the value and checks nothing, because the static type already holds it to
the set. Decode reads the base type and then checks membership: a value outside the set is one issue
under its path, worded as the checked conversion's run-time message (``a `string` of 1 byte is not
one of `"a"`, `"b"` ``), which names the value by its length and never shows its text
(`rule:types/conversion`). A row decode runs the same check, and the API document
writes the set as an `enum` beside the base type. Recursion is fine and terminates on the data, and it
is the *type* that recurses rather than only the field: an `array<T>` whose `T` is itself an
`array<…>` or an inline shape is reachable, and a decode reads every level of it. A row is the one
narrower door, because a column is a single value: `rule:core-classes/db-column-types` maps none to a
nested document, so `#[Db\Derive]` refuses a list of lists and a list of shapes at the declaration.

Two per-field overrides exist and no more: `name` renames one key or column, and `skip: true` removes
the field from the codec entirely. A property's overrides for one format are one attribute, so a second
`#[Json\Field]` or `#[Db\Field]` on it is a compile error at the second, naming the first: merging the
two would let one option silently win. A skipped property is exempt from the constructor-parameter rule
above, and a class keeping it as a parameter anyway stays well formed — it still encodes, and the
program can still build one itself. What such a class cannot do is be **decoded**: the contract names
no key for that position, and a default carried on a field cannot be read for a position that has no
field. So the refusal is of the call that asks for a whole instance out of a document or a row rather
than of the declaration, at every door that reads a derived codec. A field's default key or column
is the property's own name, exactly, with no case mapping between the two.

**The key is the field's identity**, so two fields of one format with equal keys are a compile error
at the second (`E0873`), naming the first. The encoder would write the key twice, and the decoder
would fill both properties from one value. A JSON decode reads a class only from a JSON object and
an `array<T>` only from a JSON list: `["a"]` and `{"0": "a"}` have the same entries once read, and
the decode tells them apart by what the document wrote.

**Declined:**
- **Building the object without its constructor**, the way a serializer for a language with no
  constructors does. Every decoded object would be one that never ran its class's invariants, and
  the deliberately narrow reflection bypass of `rule:classes/an-unwritten-property-read-throws`
  would become a hot path.
- **Deriving from the constructor's parameters alone.** The encoder has nothing to read for a
  parameter that is not promoted, and the two halves would come from two lists that can disagree
  in silence, which is the failure hand-written codecs already have.
- **Only parents that carry the derive count.** It is a second rule to learn, and a parent without
  the attribute would still drop its keys in silence.
- **Refusing a derive on a subclass whose parent has properties.** Every class family would need a
  hand-written codec.
- **A union of two kinds**, such as `string|int`. A decoder would have to pick a type from the value
  it read, which is what `mixed` already is, so a second way to say "any of these" adds nothing.
- **A union of two classes.** A decode cannot tell which constructor to run without a key in the
  document naming the class, and that key is a naming policy, declined below.
- **A whole-class naming policy** such as `snake_case`. It is a mode string
  (`rule:core-api/shape-rules` R11), and it makes a wire format depend on a setting rather than on
  the source being read.
