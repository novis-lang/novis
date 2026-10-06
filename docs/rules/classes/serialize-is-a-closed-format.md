`Core\Serialize::encode($x): bytes` runs the graph copy and encodes the result; `decode` runs it in
reverse. The accepted format is Novis's own, versioned and self-describing enough to be checked before
any object is built. A payload without the format marker is refused outright rather than best-effort
parsed. A payload naming a class the receiving side cannot resolve is refused, naming the class. A
payload whose recorded property set does not exactly match the target class's current declarations —
one added, removed or retyped — is refused, naming the mismatch, never coerced and never filled with a
default.

`decode` is a `tainted` sink with no launderer. The rules above close code execution but not type
confusion: a payload reconstructing a `User` with `isAdmin` set bypasses the constructor while
satisfying every check. Bytes the program produced itself carry no qualifier and decode normally;
bytes that arrived from outside are refused at compile time.

No capability grant is required, because the closed format and the no-hook rule already remove what a
grant would contain, and a hostile payload's cost is bounded by the same memory and CPU limits every
other allocation-heavy call has. What it costs is foreign data: a payload written in another runtime's
open format cannot be read at all.
