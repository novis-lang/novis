Hashing writes one algorithm and nothing else, and **no member gains an algorithm argument**. Novis
never *produces* a legacy hash; it only stops refusing to read one
(`rule:security/bcrypt-read-roster`).

An algorithm parameter is how a deployment ends up writing the weaker of two options for the lifetime
of the code that passed it, and it is a parameter no caller is in a position to choose well. Removing
it means the write side has one behaviour to reason about and the read side carries the whole
compatibility surface — which is the only place compatibility belongs, because a stored row was
written by somebody else.

The write-side parameters and their reasons live with the implementation rather than being spelled at
a call site, so raising them is one change rather than a sweep.
