A quantity carrying a unit is a type, not a number with a convention attached. A duration is a `Duration`,
never "seconds here and microseconds there"; a byte size is a `uint` count of bytes and says so.

A `sleep`/`usleep` pair split by unit is a units bug waiting for a refactor to find it, and a
signature that takes an `int` cannot tell a caller which scale it wanted. A `Duration` takes the question
out of the call site: it is constructed from the unit it is written in (`Duration::seconds`), it parses
from a written grammar (`rule:types/duration`), and every member taking a timeout takes exactly
that type. The cost is one construction at each call site that would otherwise have passed a bare integer.
