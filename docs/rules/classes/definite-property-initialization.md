Every property a class declares is assigned on every path out of every constructor the class exposes,
checked by the same flow analysis that already proves definite assignment for locals. A promoted
constructor parameter satisfies its obligation by construction; an inline default satisfies it before
the body runs; a subclass discharges what it inherits by calling `parent::constructor(...)` on every
path, and that call is trusted rather than re-derived, exactly as a callee is trusted everywhere else.
The same call discharges a property the subclass declares again when a superclass declares it with
no default and not `lateinit`, since the superclass's constructor assigns it. One the superclass gives
a default is still the subclass's to assign, unless the subclass's declaration gives one too.
A class that declares no constructor and a non-nullable property with no default is refused at the
property's own declaration, since there is no constructor body to attach the diagnostic to. Inheriting
one is not declaring one: a subclass that writes no constructor of its own runs the parent's
unchanged, and that constructor assigns nothing the subclass declared after it, so the subclass's own
properties are refused the same way.

Without it, the mistake surfaces at whichever read happens to hit the unset property, far from the
constructor that forgot it. One analysis over two binding kinds turns that into a refusal at the
cause.

A constructor also reads a property through `$this->` only where every path has assigned it, and an
inherited one only after `parent::constructor(...)`; an earlier read is refused at the read, since it
would see the zeroed slot.

Construction is two-phase, as Swift has it. Until a path has assigned every property the class itself
must assign, and has called an owed `parent::constructor(...)`, `$this` does not leave the constructor:
no method call on it, no passing or storing it, no `throw` that carries it, no anonymous function that
captures it, and no instance method called through `parent::`, `self::` or `static::`. The parent's
constructor is itself called only once the class's own properties are assigned, because it may call
a method the class overrides. Each refusal names the first property still unassigned. This is what
lets compiled code read a slot without testing it: no method the constructor calls can run while a
slot is still zero, so the check never has to follow a call. What it costs is one order a
constructor must keep, and a cycle built in a constructor (`new Pong($this)`) needs a `lateinit`
property.

The steps of `rule:classes/property-observer-pipeline` are the one place the order cannot hold: a
write in the constructor runs a hook, a guard or an observer with `$this` while later properties are
unassigned, and refusing that would refuse every observer that assigns two properties. So a class that
implements `PropertyObserver`, `PropertyGuard` or `LateInit`, or whose chain declares a hook, starts
each non-nullable property with no default unset, a promoted parameter's included, and a compiled read
of one tests it first, through that class and every class it extends. A read before the write throws
the `LogicError` of `rule:classes/an-unwritten-property-read-throws`. That costs one tag compare and
branch per such read, and one store per such slot at `new`. Every other class pays nothing.

A hooked property carries the obligation when its backing slot is read: by a plain read, because it
has no `get` hook, or by a hook that names `$this->prop` other than as the target of `=`. Each `set`
hook of such a property stores the property on every path that does not throw, or is refused at the
hook, and every write in the constructor then counts as an assignment. A property whose hooks compute
its value from other properties reads no slot and owes nothing.

One exemption is declared rather than implicit: a `lateinit` property (`rule:classes/lateinit`).
Neither copy depth reopens the gap — `clone` starts from a live object and cannot produce a less initialized
one, and the graph copy either copies a live source or refuses a payload missing a declared property
(`rule:classes/serialize-is-a-closed-format`).

What it spends is strictness PHP does not have: PHP compiles a constructor that leaves a typed property
unset on some path and fails only when that path is read, so PHP source with such a gap needs a fix,
not just a recompile.

**Declined:**
- **A runtime-only throw with no compile-time check.** It finds the mistake at a read far from its
  cause, and gives up the one analysis that already serves locals.
- **Following each call the constructor makes into the method it reaches.** A method can be
  overridden, called through `mixed` or handed to a library, so the analysis would be whole-program
  and still unsound at the first call it cannot see.
- **A silent per-type default.** A plausible zero hides the missing assignment
  (`rule:classes/no-undefined-value`).
- **An `undefined` value for the unassigned state.** It widens every declared type
  (`rule:classes/no-undefined-value`).
