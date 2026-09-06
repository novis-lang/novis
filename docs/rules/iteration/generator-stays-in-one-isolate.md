A generator object is not serializable, does not cross a `spawn`, `spawn worker` or `spawn script`
boundary, and does not `clone`. Each attempt is the same refusal any value that cannot be copied
soundly already gets.

The state-machine lowering (`rule:iteration/generators`) makes a generator an ordinary object with
ordinary fields, so the refusal is not a representation limit: those fields are a compiler-chosen
encoding of a suspended program point, and resuming that point in another isolate — or in a second
copy inside this one — has no meaning to give.

Not yet enforced: nothing in the runtime's copy or spawn paths refuses a generator today.
