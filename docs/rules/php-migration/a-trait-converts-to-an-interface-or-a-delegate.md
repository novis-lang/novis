Every PHP trait shape maps to one of five outcomes. Two are mechanical, one is mechanical but
flagged, and two need a human decision, named rather than silently attempted.

1. **Stateless trait** (methods only). `trait T` becomes `interface T` verbatim — a method with a
   body becomes a default (`rule:classes/interface-default-methods`), an `abstract` method a plain
   interface method — and every `use T;` becomes `implements T`. A `$this->helper()` call in the
   trait body that targets a method not on its abstract list is added to the interface as an
   abstract method, making the trait's implicit contract explicit; still mechanical, since every
   `$this->` call site is known at the syntax level.
2. **Stateful trait** (declares a property). An extracted interface carrying the trait's public
   signatures, plus a generated `{Trait}Impl` class owning the property and the original bodies
   verbatim; `use Timestamps;` becomes `implements Timestamped by $timestamps`
   (`rule:classes/delegation-by-field`) with a field and a constructor assignment, merged into an
   existing constructor. Mechanical, but it reshapes the surrounding code enough to be flagged for
   review.
3. **`insteadof`**, either shape, becomes an explicit override calling the winner by name:
   `A::hello()` in the stateless case, `$this->a->hello()` in the delegated one.
4. **A trait `static` property** has no destination — it is the ambient, silently duplicated state
   `rule:statements/static-is-a-member-modifier` forbids — so a `TODO` names it, and a human chooses
   one owning class or delegated instance state.
5. **A trait method calling an unrelated method of its consuming class** has no mechanical
   translation once the trait is an independent object: the `Impl` holds no reference back. A `TODO`
   names the call site, and the human passes whatever callback or interface the tracker needs into
   its constructor.
