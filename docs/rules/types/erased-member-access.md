Three receivers are erased: plain `object`, a shape type asked for a name it does not list, and
`mixed` — the widest, because it defers not only *which* class is behind the handle but *whether there
is one*. Every receiver whose **declared** type can hold no object — a scalar, an `array<T>`, a union
naming no single class — is refused where it is written (`E0495`) instead, because a type that has
already answered the question does not get to ask it again at run time.

For all three:

- **Read:** a checked, catchable throw if the concrete instance does not have that name — never a
  silent value, never PHP's warning and a `null`. A receiver whose tag turns out not to be an object
  is one more catchable throw, worded as PHP words its warning.
- **Write:** the same missing-name throw, plus a check of the incoming value against the field's
  *real*, concrete declared type, throwing on a mismatch. A write through an erased view can **never
  create a field**.
- **Call:** a call through an erased receiver is dispatched on the value's own descriptor when it
  runs. What cannot be deferred is refused: taking a *method reference* off an erased receiver has no class
  present to read a callee from (`rule:types/callable-values`).

Both throws are ordinary `Throwable`s propagated by checked return (`rule:errors/propagation`), never
routed through the fatal escalation ladder (`rule:errors/escalation-ladder`). Per-property hooks and a
declared property observer behave here exactly as they do anywhere, because this is one implementation
and not a family of them — `rule:types/property-key-access` and `rule:types/mixed-subscript` are the
same access one step along.
