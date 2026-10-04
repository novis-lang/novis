After the subject come the required arguments in dataflow order, and then **at most one** trailing optional
anonymous object (`rule:types/object-top`) whose shape is declared as a `type` alias. There are no `bool`
flag parameters, no `int` bitmasks, and no positional optional tail longer than one.

The bag is the home of every optional knob because it is named, order-free and structurally checked, and a
bag written entirely from compile-time constants folds to a constant. It also flattens at the call site
into one argument per declared field (`rule:core-api/shape-flattens-at-the-abi`), so nothing is allocated
to carry it. A bag field may be nullable, and where it is, leaving the key out and writing `null` into it
are two different requests (`rule:core-api/omission-is-not-a-written-null`).

The cost is that a bag's keys are declared and fixed: a member that must take a key whose *name* is chosen
at run time needs a second member taking a `string`, which is why `Core\Uri` carries both `with` and a
query-parameter pair rather than one member doing both.
