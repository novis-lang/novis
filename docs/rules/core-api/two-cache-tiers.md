Cross-request state is reached through **two members with two contracts**, not one member with a flag. The
local tier is per-core and in-process: any entry may be absent at any time for any reason, and a write on
one core is not visible on another. The shared tier is a real store over the network, coherent across cores
and machines, and gated by the capability that names the store an operator configured. Neither member takes
an argument, so there is no flag to have written.

A program that would be incorrect if a read returned nothing is using the wrong tier, and the whole value of
two members is that the choice is made in the source and visible in review. One name covering two different
guarantees invites using the weaker one by accident — the same reason a generic sanitizer is refused.

The local tier needs no capability, because a capability is checked at the door to an *effect* and this tier
has no door: nothing leaves the process, no name is resolved and no file is opened. What is left to bound is
footprint, and a configured size cap is the instrument for that; a boolean grant is not one, and adding it
would price the tier as an authority question every deployment then has to answer.
