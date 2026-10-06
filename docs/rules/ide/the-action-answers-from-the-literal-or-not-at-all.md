Three refusals, and each is a soundness question rather than a difficulty one.

**No literal, no action.** `array<mixed> $body = Core\Json::decode($raw);` gets nothing. Neither does a
`Core\Request::query()`, a `Core\Script::args()` or any other value that arrives from outside — those are
`array<mixed>` on purpose (`rule:types/unions-and-mixed`, `rule:statements/no-host-populated-variables`).
The shape of one payload someone looked at does not bound the next request's, and narrowing there would
convert a runtime check into a compile-time assumption that is not true. This is security, not
ergonomics, and a developer may not point the action at such a value on the grounds that they know their
data.

**A `mixed` element poisons the answer, and a no-op is not offered.** An element whose own type is
`mixed` — including a spread of one — makes the union `mixed`, so the synthesis yields `array<mixed>` and
the action stays hidden rather than proposing the annotation that is already written.

**Everything else is answerable**, and that is a property of the language rather than of the
implementation: because every binding site declares a type, an element that is a variable or a call has a
type the checker already holds. There is no PHP-style guess anywhere in this action.

Narrowing from writes made *after* the declaration is not offered: it needs the join over every write
reaching the declaration and a generator that picks between answers, which
`rule:ide/a-code-action-writes-only-what-is-already-determined` refuses.
