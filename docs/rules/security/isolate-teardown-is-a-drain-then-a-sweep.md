When an isolate ends — returning, throwing, breaching a limit, or being cancelled — its context's
roots are released through one iterative worklist. A graph nested a million deep costs no stack, the
cost is bounded by what is **live** at that moment rather than by what was ever allocated, and every
native drop runs on the way. A cancelled isolate takes the same path and no other.

The drain frees only what the refcounts say is dead, and a cycle's members hold each other above zero.
An object is the one shape that can close a cycle, so the drain is followed by a **sweep** over the
context's intrusive live list. The sweep first tallies, per member, how many references come from
another member's field slot or from an array that member solely owns; a member the tally does not exactly account for is reachable from outside
and is left alone, along with everything under it. Freeing memory somebody still holds is a
use-after-free, so security decides a question memory footprint would have answered the other way.
A survivor is taken off the list, because the list dies with the context and the object does not.
Only the remainder is dismantled, through the same worklist, so native teardown runs there too.

The sweep is an obligation, not an option. Without it a cyclic object graph would be retained for the
life of the process, and in the server that is a leak growing with requests served. The tally reads
an array only when one member solely owns it; the elements of a shared array read as held from
outside, so a cycle whose only closing edge runs through one survives the sweep rather than risk a
use-after-free. `crates/nvs-runtime/src/object.rs` is the mechanism's one home.

- **Memory:** reclamation is O(live values), not the O(1) of unmapping a region; in exchange the drain
  is the runtime's only reclamation path, so it is the only one that can be wrong. The sweep spends two
  pointers per live object and a few non-atomic stores at each object's allocation and death.
