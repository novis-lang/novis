A task's recursion limit is armed from the base and length of the stack it was actually handed, not
asserted from a compiled-in ceiling. The host allocated that stack and knows its bounds exactly, so
the limit it arms is a fact about the memory the task is standing on.

That is what makes deep recursion inside a task a thrown, catchable error rather than a walk into a
guard page. The width of the stack is set by the same arithmetic: an unwinding reserve already sits
between the soft recursion limit and the hard floor (`rule:errors/on-limit`), so a stack has to be
several times that reserve before the two numbers are coherent — which is why a 64 KiB stack is
rejected on arithmetic rather than on taste.
