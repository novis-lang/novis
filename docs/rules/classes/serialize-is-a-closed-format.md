`Core\Serialize::encode($x): bytes` runs the graph copy and encodes the result; `decode` runs it in
reverse. The accepted format is Novis's own, versioned and self-describing enough to be checked before
any object is built. A payload without the format marker is refused outright rather than best-effort
parsed. A payload naming a class the receiving side cannot resolve is refused, naming the class.
`decode` refuses every class `encode` refuses, with the same sentence, by asking the function
`rule:classes/graph-copy` names once the class resolves. It asks before it compares a property or
makes an object, so a refused payload builds nothing and its error does not print the class's layout.
The cost is a few flag reads and one scan of the class's `secret` flags per decoded object. A
payload whose recorded property set does not exactly match the target class's current declarations —
one added, removed or retyped — is refused, naming the mismatch, never coerced and never filled with a
default.

Every value the decode builds is checked before the result is returned. A field's value is checked
against the field's declared type by the same check an erased write makes
(`rule:types/erased-member-access`): its tag, its class and an array's elements. A `string` node and a
string array key must be UTF-8, and a `bytes` node is not checked. A payload that fails either check is
refused, naming the class and the property, so no typed read can see a value of the wrong kind,
whatever the bytes are. The fields are checked once the whole graph is built, because a
back-reference can reach an object whose later fields are not filled yet. The cost is one check per
field on a decode and nothing in memory.

A `Core` class's slots are checked the same way. Every `Core` class with instances and no host handle
has one row in the roster `SLOT_TYPES`, beside the class descriptors (`nvs_stdlib::instance`). A row
is the class and either one type per slot, in the type language a member's signature uses, or a
reason sentence. A slot's type may be `mixed`, a union, or a `Core` enum, which admits only that
enum's case values. A class with no slots has an empty row. The roster fills the tables the check
reads, and a debug build checks each `Core` instance it builds against its row. A `Core` class with
no stated slot types is not written and not read: `encode` and `decode` both refuse it, with one
sentence, before an object is made. That is the default for a class added later, so it is refused
until it says what it holds, and a test that walks the registry fails until it has a row.

A row is a reason when a reader relies on more than each slot's type. That is a slot that is an
index, a length or a key into another slot or into a table of the request or the core; the state of
a walk, a stream or a queue still running; text written out with no check, such as `Core\Html\Markup`;
key material; and a value a `Core` member checked when it made the instance and its readers do not
check again. A reader that checks a value again at each use and throws on a bad one does not rely on
it, so a `Core\Time\Date` with a month of 13 is read back and its first member throws. Such a class
is refused with its reason, which ends the refusal's sentence. A live crossing does not
check slots: it copies an object a `Core` member made, and with `decode` checked and `construct`
closed, no `Core` instance has a slot of a type its class does not state. The cost is one tag test
per slot per decoded `Core` object, and one small table per `Core` class, built once per process. A
program that decodes nothing pays nothing per request. The usability cost is that a class whose row
is a reason does not go through the cache, the session or `Core\Serialize`.

Six doors read bytes, and every one decodes through this check:

| Door | Who wrote the bytes |
|---|---|
| `Core\Serialize::decode` | the program, which hands them over |
| the shared cache tier | whoever can write the store, which is outside the process |
| the session record | whoever can write the session store: the data folder's file or the shared store |
| the memory cache tier | this process's `encode` |
| `spawn` placed on another core | this process's `encode` |
| a topic message | this process's `encode` |

The first three read bytes a program or an outsider can write. The last three read bytes nothing
forged, and they pay the slot check too, because one decoder is checked in one place.

`decode` is a `tainted` sink with no launderer. The rules above close code execution but not type
confusion: a payload reconstructing a `User` with `isAdmin` set bypasses the constructor while
satisfying every check. Bytes that arrived from outside are refused at compile time. `encode` is an
encoder under `rule:security/taint-propagation`: over a value that provably carries no outside text it
answers plain `bytes`, which decode normally, and over one that may it answers `tainted bytes`. A
program decoding what it encoded itself says so once, with `Core\Taint::assertTrustedBytes`
(`rule:security/assert-trusted`).

No capability grant is required, because the closed format and the no-hook rule already remove what a
grant would contain, and a hostile payload's cost is bounded by the same memory and CPU limits every
other allocation-heavy call has. What it costs is foreign data: a payload written in another runtime's
open format cannot be read at all. Reading one would be a separate wire-format feature, never a
loosening of this format.

**Declined:**
- **PHP's open wire format, for converted code.** Its looseness — any well-shaped payload naming any
  resolvable class — is an attack surface before any hook runs, and the closed format removes it.
- **A capability grant for `decode`.** `script.spawn` is a grant because what it builds runs
  (`rule:security/script-spawn-capability`); what `decode` builds is inert data.
- **`decode` passing taint on to its result.** The danger is the reconstructed graph itself, not a
  string that later reaches an output sink, so `decode` is a sink rather than contagious.
- **A field of slot types on every `Core` class literal.** Most `Core` classes have no slots, and one
  roster beside the descriptors says it in one place.
- **A hand-written list of `Core` classes that are "data".** It states nothing `decode` can check.
- **Each `Core` reader testing its slot before it returns it.** That is one check per member, and a
  new member would have to remember it. `decode` checks each slot where the bytes come in.
- **Skipping the slot check for bytes this process wrote.** That is a second decoder to keep in step.
- **A reason row for every `Core` class whose slots a check could fail on.** It would take the dates
  and times out of the cache. A reader that checks again and throws is enough for those.
