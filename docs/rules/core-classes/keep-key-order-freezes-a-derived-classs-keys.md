`#[Core\Json\KeepKeyOrder(reason: string)]` is a compiler-recognized attribute
(`rule:core-classes/derive-attribute`'s closed list), allowed on a class only, with one required field,
`reason`. It says the class's JSON key order is read by somebody: a consumer that wants a `type` key
first, an ETag, a signed or cached document.

**It compiles only on a class that also carries `#[Core\Json\Derive]`.** Anywhere else it would do
nothing, so it is a compile error at the attribute, naming the missing derive. The codec itself does
nothing new with it: the encode order is the declaration order
(`rule:core-classes/derive-property-list`), as it always is.

**What it freezes is the order the tools write** (`rule:tooling/class-members-have-one-order`). A
derived class's field order is its instance properties in written order, with the promoted constructor
parameters at the constructor's position. Under `KeepKeyOrder`, the instance properties keep their
written order, and every other member of the class is ordered as usual. The ordered layout puts the
constructor after every property, so when a promoted parameter comes before a declared property in the
field order, no member order keeps it: the class is then left exactly as written, and `nvs code order`
prints one note naming it.

An empty `reason` compiles. Review judges a reason, and a check cannot.

It is named for JSON because `Core\Json\Derive` is the only derived codec whose output order a consumer
reads: `Core\Db\Derive` matches columns by name.

**Declined:**
- **A generic `KeepOrder`.** The compiler could not check that it sits where an order means something.
- **An `order` field on `Core\Json\Field`.** It is a second way to write the order the declaration
  already gives.
- **Refusing an empty `reason`.** Review judges a reason, and a check cannot.
