A `Core` member whose `string`/`bytes` parameter carries no classification **rejects a tainted
argument**. *Contagious* is a thing an author writes rather than a thing an author gets by forgetting,
and the failure direction inverts: before, a member nobody classified accepted tainted data at run
time in production; now it refuses where the call is written.

The classification lives once, beside the parameter list and the return type in the member registry,
and a member with an unclassified parameter **fails the library's own test suite** — the default is
what a program sees, the test is what stops such a member from shipping at all. Userland was already
fail-closed, because a plain `string` parameter of a user's function refuses a tainted argument, so the
library's own table was the one fail-open surface, and flipping its default was one pass over one table
rather than a language change. The pass marks most parameters *contagious*, because most genuinely
are, and it never recurs: the next member added is safe until its author says otherwise.

Every mark is checked before the program runs and erased before code is generated, so the default
costs nothing on the request path: no representation, no branch.

A **union parameter** carries the mark its arms declare — `Pattern|string` is a sink because the
`string` arm is written as one — and carries none where its arms disagree or where an arm that could
hold a mark does not. The classification is still written once beside the parameter, on the arm that
has a cell for it, and the refusing default still covers everything nobody classified.

A mark that admits `tainted` admits a union carrying it, arm by arm, and the contagion is read back
out with at least that reach. `Core\Env::get` returns `?tainted string`, so `$name ?? "default"` is a
`string|tainted string`, and that is the shape every environment read arrives in: a launderer that
took only the atom would refuse the one form its own source produces. The admission does not weaken,
because each arm is narrowed and then compared, so one tainted arm at a sink still refuses the whole
union. The two directions are deliberately asymmetric: the answer that decides
whether to *set* the qualifier on a result reaches further than the narrowing that decides whether to
*admit* an argument, because reaching too far over-taints in the first case and leaks in the second.

**Declined:**
- **Keeping *contagious* as the default and stating the sink predicate as guidance.** The predicate was
  already derivable, and a member taking a format template was still added unmarked. A rule held by
  attention fails exactly when nobody is paying it.
