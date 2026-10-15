At `Category::find(...)` and `$category->find(...)`, a `static` inside a parameter type —
`property<static>`, `propertyPath<static>`, `string<propertyPath<static>>` — is the receiver's class,
the way an exact `: static` return already is (`MethodSig::returns_static` in
`crates/nvs-types/src/signatures.rs`). A `static[]` and a `?static` return are bound the same way,
which `writes_static_return` in the same file leaves out today, so
`Category::all()` declared `: static[]` is a `Category[]` at the call.

Inside the method body a `property<static>` or `propertyPath<static>` value may be applied only to a
receiver whose type is `static`, converted with `as string`, or passed on. The body does not know which
subclass called it, so a key of `static` read on `self` could name a property only the subclass has.

**Fallback:** if the checker cannot keep `static` apart from the declaring class inside the body, the
body may only convert such a value with `as string` or pass it to `Core\Str::parts` or
`Core\Reflect::steps` (`rule:core-classes/str-parts-and-reflect-steps`), and this rule says so when it
ships.

This extends `rule:statements/static-is-a-member-modifier`, under which `static` binds late to the
called class everywhere it names a type; a parameter and an array return are the two positions that
binding does not reach yet. A base class that offers `find(propertyPath<static> $sort)` once then
checks each subclass's own fields at each call, with no override per subclass.

- **Memory:** nothing at run time. The binding is the checker's, at each call it resolves.

**Declined:**
- **Writing the class into each subclass's override.** It is the boilerplate late binding exists to
  remove, and a missed override checks against the base class's fields.
- **Binding `static` in a parameter at run time.** The call site already names the class, so the check
  belongs where the call is written.
