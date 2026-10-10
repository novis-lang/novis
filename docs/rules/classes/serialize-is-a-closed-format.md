`Core\Serialize::encode($x): bytes` runs the graph copy and encodes the result; `decode` runs it in
reverse. The accepted format is Novis's own, versioned and self-describing enough to be checked before
any object is built. A payload without the format marker is refused outright rather than best-effort
parsed. A payload naming a class the receiving side cannot resolve is refused, naming the class. A
payload whose recorded property set does not exactly match the target class's current declarations —
one added, removed or retyped — is refused, naming the mismatch, never coerced and never filled with a
default.

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
