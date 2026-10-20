An instance property a class declares again over one a superclass declares keeps the superclass's
exact type. Otherwise it is `E0866`, at the redeclaration. Both declarations name one slot, and the
superclass's code reads and writes it at the superclass's type: a narrower type lets the superclass
store a value the subclass reads as another class, and a wider one lets the subclass store a value
the superclass reads as another class. "Exact" is assignability both ways, without the `int` →
`float` row, so `?Animal` and `Animal|null` are the same type and `float` over `int` is refused.

It holds for every way a property is declared: a plain property, a promoted constructor parameter,
a `lateinit`, an `array<T>` and a hooked property. A hooked property's `set` parameter already
accepts the property's own type (`rule:classes/property-hooks`), so its hooks keep the superclass's
type with it. A redeclaration at the same type compiles, which is how a subclass overrides a hook.

**A `readonly` property is not declared again**, on either side: both constructors would write it,
and `rule:classes/readonly-is-written-once` allows one write. That is `E0866` too.

A static property is outside the rule. Each class names its own storage, and `static::$x` is refused
(`rule:statements/static-is-a-member-modifier`). An interface declares no property
(`rule:classes/interfaces-declare-no-state`), so only the superclass chain is walked.

`rule:statements/static-is-a-member-modifier`'s `E0833` is the same hazard for a constant, and it
allows any assignable type there because a constant is only read, never written through the parent.

Declined: a narrower property type with a check on each write through the parent. It is sound, but
it is a second mechanism with a run-time check on every write of a redeclared property.
Declined too: PHP's rule, which allows the same as this one, except that PHP keeps the parent's
`private` property as a separate slot. Novis gives a redeclared name one slot whatever its
visibility, so a `private` redeclaration is held to the same type.

`crates/nvs-types/src/conformance.rs`'s `check_property_redeclarations` is the check.
