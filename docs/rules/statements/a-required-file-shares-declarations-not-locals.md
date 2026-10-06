Every name a required file *declares* — a class, an interface, an enum, a `type` alias — is visible to the
requiring file exactly as if it had been pasted in. A local variable does not cross in either direction:
each file's top-level body is its own frame and is checked on its own, so a required file's `$x` is not the
caller's and the caller's is not the required file's.

The required file's own top-level statements run where the `require` is written, in source order, once per
time the site is reached — ordered against the statements around it rather than hoisted the way its
declarations are.

A required file never sees the includer's locals. A shared variable scope
would need one flow-sensitive definite-assignment analysis spanning a graph whose shape a `require` inside
an `if` decides at run time. A program that relied on it passes what it means as a constructor argument or
a static.
