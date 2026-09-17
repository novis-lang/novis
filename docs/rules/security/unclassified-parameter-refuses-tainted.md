A `Core` member whose `string`/`bytes` parameter carries no classification **rejects a tainted
argument**. *Contagious* is a thing an author writes rather than a thing an author gets by forgetting,
and the failure direction inverts: before, a member nobody classified accepted tainted data at run
time in production; now it refuses where the call is written.

The classification lives once, beside the parameter list and the return type in the member registry,
and a member with an unclassified parameter **fails the library's own test suite** — the default is
what a program sees, the test is what stops such a member from shipping at all.

A **union parameter** carries the mark its arms declare — `Pattern|string` is a sink because the
`string` arm is written as one — and carries none where its arms disagree or where an arm that could
hold a mark does not. The classification is still written once beside the parameter, on the arm that
has a cell for it, and the refusing default still covers everything nobody classified.

A mark that admits `tainted` admits a union carrying it, arm by arm, and the contagion is read back
out with at least that reach. The two directions are deliberately asymmetric: the answer that decides
whether to *set* the qualifier on a result reaches further than the narrowing that decides whether to
*admit* an argument, because reaching too far over-taints in the first case and leaks in the second.
