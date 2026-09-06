A request and everything it spawns form one tree, and limits are accounted against the tree's root. An
isolate receives no budget of its own *in addition* to its parent's; it spends the parent's. A request
that spawns fifty isolates does not get fifty times the ceiling, it gets one ceiling to divide, so the
worst case of a process is still in-flight requests times the hard memory limit.

`limits:` at the spawn site sets a **sub-cap**: tighter than what remains, never wider. Child tasks
count against the root's `max_tasks`, child output against its `max_output`, child CPU against its
`cpu_time`. A fork bomb is bounded by arithmetic rather than by a heuristic, and `max_script_depth`
bounds nesting so runaway recursion is diagnosed as such instead of as a memory limit.

The cap attaches as **arithmetic at the allocation that asks**, not as the bound of a mapping. An
isolate that asks for more than its share is refused at the call that asked, with a limit report
naming the directive, and never by a page fault at an address nobody chose
(`rule:errors/on-limit`).
