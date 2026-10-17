Fields are the class's own declared properties, in declaration order, private ones included.
Visibility is an access-control decision and has no business deciding a wire format; declaration
order fixes encode order, so output is byte-deterministic across runs and machines.

**Every non-skipped field must also be a constructor parameter of the same name and type**, or the
derive is a compile error at the attribute. For a class written with promoted parameters the two
lists are literally the same declaration, so this costs nothing. It is worth its cost because a
decode is an ordinary `new`: the generated decoder fills locals and calls the constructor, so a
decoded object is indistinguishable from a hand-built one and every invariant the constructor
establishes still holds. A `lateinit` property cannot be a field, being by definition not
constructor-assigned; `readonly` is unaffected, because the constructor assigns it like any other.
Because the two lists are checked to agree at compile time, `fromJson(toJson($x))` rebuilding `$x`
is a property of the mechanism rather than of the care taken writing two functions, and a renamed
property breaks loudly: every reader of the old key gets a missing-field issue with its path.

A field's type must be codec-reachable — a scalar, one of the named `Core` value types, an enum, an
inline shape, an `array<T>` or `?T` of one of those, or another class that itself has a codec.
Anything else is a compile error at the field. Recursion is fine and terminates on the data, and it
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

**Declined:**
- **Building the object without its constructor**, the way a serializer for a language with no
  constructors does. Every decoded object would be one that never ran its class's invariants, and
  the deliberately narrow reflection bypass of `rule:classes/an-unwritten-property-read-throws`
  would become a hot path.
- **Deriving from the constructor's parameters alone.** The encoder has nothing to read for a
  parameter that is not promoted, and the two halves would come from two lists that can disagree
  in silence, which is the failure hand-written codecs already have.
- **A whole-class naming policy** such as `snake_case`. It is a mode string
  (`rule:core-api/shape-rules` R11), and it makes a wire format depend on a setting rather than on
  the source being read.
