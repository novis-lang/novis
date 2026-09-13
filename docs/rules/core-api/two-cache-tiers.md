Cross-request state is reached through **a member per tier, each with its own contract**, not one member
with a flag. The local tier is per-core and in-process: any entry may be absent at any time for any reason,
and a write on one core is not visible on another. The process tier is one store per serving process,
coherent across every core of it, in memory only and gone when the process ends
(`rule:concurrency/the-process-tier-is-one-store-per-process`). The shared tier is a real store over the
network, coherent across cores and machines, and gated by the capability that names the store an operator
configured. No member takes an argument, so there is no flag to have written.

A program that would be incorrect if a read returned nothing is using the wrong tier, and the whole value of
a member per tier is that the choice is made in the source and visible in review. One name covering
different guarantees invites using the weaker one by accident — the same reason a generic sanitizer is
refused. A tier is added by adding a member, which is why this rule's id counts two while its title does
not: an id is the fragment's path and a path does not move.

Neither weak tier needs a capability, because a capability is checked at the door to an *effect* and neither
has a door: nothing leaves the process, no name is resolved and no file is opened. What is left to bound is
footprint, and a configured size cap is the instrument for that — `[cache.local] max_size` per core and
`[cache.process] max_size` per process; a boolean grant is not one, and adding it would price a tier as an
authority question every deployment then has to answer.
