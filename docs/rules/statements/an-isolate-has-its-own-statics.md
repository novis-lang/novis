An isolate's context is built with a **statics base of its own**, and that single word is the whole of "a
child cannot read or write a parent static". Globals, class statics and runtime-defined constants are fresh
per isolate.

Compiled code loads a static inline through that base, so the same compiled function reads a different
store depending only on which context it runs under — which is why sharing compiled code between isolates
costs nothing and needs no per-isolate compilation.

A task is the other construction, and deliberately not the same one: a task shares the request, so it
aliases the request's static-property base rather than freshening it, and a child with a store of its own
would give one request two copies of every static. Collapsing both into one helper with a flag would leave
the next field added to the context classified by whoever adds it rather than by whoever needs it fresh.
