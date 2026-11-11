Three receivers are erased: plain `object`, a shape type asked for a name it does not list, and
`mixed` — the widest, because it defers not only *which* class is behind the handle but *whether there
is one*. A receiver whose **declared** type answers the question is never erased: a type that can hold
no object is refused where it is written (`rule:types/a-declared-type-answers-before-the-program-runs`),
and a union of classes resolves a member against every class
(`rule:types/a-member-every-class-of-a-union-declares`).

For all three:

- **Read:** a checked, catchable throw if the concrete instance does not have that name — never a
  silent value or a `null`. A receiver whose tag turns out not to be an object is one more
  catchable throw.
- **Write:** the same missing-name throw, plus a check of the incoming value against the field's
  *real*, concrete declared type, throwing on a mismatch. The check tests what `is` tests: an enum
  field takes a case value of that enum, a `callable` field takes a callable, a written signature
  takes a callable that fits it, and a field of a `Core` class takes an instance of that class. A
  write through an erased view can **never create a field**. An anonymous object has no declared class, and its slot carries no declared
  type, so its write is checked against the value the field has now: the new value has the same
  exact type, an object's class is the current one's or a subclass of it, a nested anonymous object
  passes the same check field by field, and each array element has the shape of some current element.
  This refuses some writes the declared type allows: a `{pet: ?Dog}` field that is `null` takes only
  `null`, an empty array takes only an empty array, and an absent optional field takes nothing. It
  spends nothing per object, which a declared type carried by each anonymous object would.
- **Qualifiers:** the run-time check of a write and of a call's arguments tests representations,
  and neither `tainted` nor `secret` has one. So a value that carries `tainted`, `secret` or `mixed`
  does not compile as the value of a write through an erased receiver, as an argument of a call
  through `mixed`, or as a value argument of `Core\Reflect\ClassInfo::set`, `call` and `construct`
  (`E0895`). An array literal at `call` or `construct` is judged element by element. A `secret`
  value written through `mixed` is already the `mixed` refusal of `rule:security/secret-qualifier`,
  and reports nothing more. The form that compiles narrows the receiver with `$x is ClassName`.
- **Visibility:** an erased receiver is outside every class, so a read or write of a property that
  is not `public` throws, as a call of a method that is not `public` through `mixed` does. That
  holds inside the property's own class too, whose code reaches the property through a variable of
  its type. A `??` over such a read throws as well, and `is` with a shape that names the property
  gives `false`. `Core\Reflect\ClassInfo` reaches further, because it asks the visibility from its
  own call site (`rule:security/reflection-enforces-visibility`).
- **Call:** only `mixed` defers a call. A call through `mixed` is dispatched on the value's own
  descriptor when it runs; a call through a plain `object` or a shape is `E0477`, as the paragraphs
  below say. What cannot be deferred is refused: taking a *method reference* off an erased receiver
  has no class present to read a callee from (`rule:types/callable-values`).

Both throws are ordinary `Throwable`s propagated by checked return (`rule:errors/propagation`), never
routed through the fatal escalation ladder (`rule:errors/escalation-ladder`). The run-time name
lookup, and the type check on a write, is a latency cost a statically known receiver never pays; only
code that erased its receiver's type pays it. Per-property hooks and a
declared property observer behave here exactly as they do anywhere, because this is one implementation
and not a family of them — `rule:types/property-key-access` and `rule:types/mixed-subscript` are the
same access one step along.

**A method call needs a class label, so a receiver that names no class is refused where it is
written (`E0477`).** `object` erases to exactly the pointer a named class does, so nothing below the
checker wants the label for representation. What wants it is resolution: `$o->m(...)` has no signature
to check its arguments against and no return type for the position it sits in. There is no `__call`
to fall back on (`rule:classes/no-call-magic`), and a dynamic call with run-time-checked arguments is
granted to bare `callable` alone. `E0477` is one code for every receiver that names no class, because
it is one mistake: plain `object`, a shape, a union with a member that is not a class, and a type that
can hold no object at all, such as a scalar, an `array<T>` or a `void` call's result. A union of
classes is resolved against every class instead (`rule:types/a-member-every-class-of-a-union-declares`). Only the help splits:
narrow the receiver (`is` proves the class, `as ClassName` converts to it), convert it, or nothing at
all for a value that does not exist. The property half splits a deferral off from `E0495` and the call
half does not, because a property read has a name-keyed fetch to defer to and a call has nothing.
`mixed` is the one receiver left out, since `rule:types/unions-and-mixed` makes it the one unchecked
position.

**A call through `mixed` is checked against each parameter's declared type.** The receiver's
descriptor carries, for each method, its arity, the representation each parameter requires, and the
rest of each parameter's type as a property slot carries it. The representation is tested by the same
per-argument check a bare `callable` call runs, so the `int`-into-`float` widening
(`rule:types/implicit-widening`) is applied in one place. The rest is tested by the check a write by
name runs, so an argument passes what `$x is T` passes: an object of an admitted class, an array whose
every element fits, an enum value one of the enum's cases has. A variadic parameter receives one array
and each element is checked. A `covariant` parameter is checked by the method's own entry test.
`Core\Reflect\ClassInfo`'s `call` and `construct` and an isolate's entry run the same check. Nothing is
converted on the way in or out: the arguments are already tagged values, and the result comes back
tagged for the `mixed` the call's own type is. Every failure is a catchable throw on
`rule:errors/propagation`'s error edge, worded as the diagnostic that names the same mistake where a
static type shows it: a receiver whose tag is not an object (`E0477`), a class with no such method
(`E0405`), and an argument count or type the callee does not admit (`E0402`, `E0401`), as a
`LogicError`. Four more shapes throw instead of dispatching. A non-`public` method throws, since a
`mixed` receiver is outside every class. A `Core`-owned method throws, since its native convention
borrows argument 0 where a compiled method owns its parameters. A `static` method throws, since it has
no object to be called on, and an isolate's entry that is not `static` throws for the same reason. A
method with an `inout` parameter throws, since a call by name has no variable to write the value back
to.

- **Memory:** the method rows are per class, once per process, and nothing per instance or per call:
  two small vectors and two bits per method beside the arity and the tag word. A call by name that
  succeeds allocates nothing. Its receiver and up to sixteen arguments sit in a stack array of 272
  bytes, the argument list of `ClassInfo::call`, `ClassInfo::construct` and a `mixed` call is read into
  another of 128 bytes, and a refusal's sentence is formatted only when it is thrown. A list longer
  than sixteen is read into a heap vector, and the tag word then refuses the call.
- **Latency:** a statically typed call pays nothing. A call through `mixed` pays a tag test on the
  receiver, one binary search of the method table by name, a shift, a mask and a compare per argument,
  one class walk per class-typed argument and one element walk per typed-array argument.

**Declined:**
- **Dispatching a method call through `object` or a shape.** Refusing is the reversible choice: a
  later decision can turn `E0477` into dispatch, while a program that already dispatched could not be
  taken back.
- **A compiled thunk per method for a call through `mixed`.** It would write the tag rules a second
  time in the code generator, add a second frame on the erased path that still needs the same name
  lookup, and spend a compiled function per method in every unit whether any `mixed` receiver exists
  or not.
