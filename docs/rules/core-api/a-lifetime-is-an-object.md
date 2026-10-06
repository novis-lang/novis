Anything with a lifetime is an object. There is no `resource` atom in any `Core` signature, no integer
handle, and no `$link`-first calling convention; a file, a connection, a compression stream and a hash
context are all objects with methods.

A member of another class may **take** such an object where it neither opens nor closes it and the class
it sits on owns a grammar rather than the resource — `Core\Csv::rows(Core\IO\File $file, …)` reads records
forward off a handle `Core\IO::open` has already checked a capability for, and putting a format's grammar
on the file class instead is what the paragraph below argues against. Opening, closing and positioning
stay members of the object, so the handle-first convention is still refused for everything that is an
operation *on* the resource.

A handle has nowhere to enforce a capability and nothing to hang an API on, so every operation on it
becomes a free function taking the handle first, and the object API that follows later sits beside it
rather than replacing it (`rule:core-api/one-paradigm-per-operation`). An object has both
a place for the capability check and a place for the methods. The `resource` atom survives in the type
grammar (`rule:types/grammar`) only for opaque handles an extension supplies, and `Core` never produces
one.
