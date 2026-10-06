When an isolate ends — returning, throwing, breaching a limit, or being cancelled — its context's
roots are released through one iterative worklist. A graph nested a million deep costs no stack, the
cost is bounded by what is **live** at that moment rather than by what was ever allocated, and every
native drop runs on the way. A cancelled isolate takes the same path and no other.

The drain frees only what the refcounts say is dead, and a cycle's members hold each other above zero.
An object is the one shape that can close a cycle, so the drain is followed by a **sweep** over the
context's intrusive live list. The sweep first tallies, per member, how many references come from
another member's field slot; a member the tally does not exactly account for is reachable from outside
and is left alone, along with everything under it. Freeing memory somebody still holds is a
use-after-free, so security decides a question memory footprint would have answered the other way.
