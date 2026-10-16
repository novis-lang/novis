A member that needs no capability pays nothing at all: no table lookup, no branch, no field on its
row, no code emitted at its call sites. That falls directly out of the check living inside a function
such a member never calls, and the declaration being data no execution path reads.

A member that does need one pays, on top of a syscall: one enum-indexed field read on the snapshot it
already holds, and for a scoped capability one canonicalisation of the argument plus a component-wise
prefix compare per granted root. The canonicalisation is on the order of a microsecond and is dwarfed
by the open it precedes.

Every member that reaches this check is by construction about to make a syscall, so **the check is
never on a hot path**. That is the whole latency argument, and it holds because of *where* the check
is rather than because of how it is written.
