A parking socket implements nothing but `std::io::Read` and `std::io::Write`. Its signature says
nothing about suspension, and no caller up the chain is marked: a helper reads it with the same call
it would use on a blocking socket, and an unmodified protocol implementation — a TLS session, a
sans-IO codec — drives it without knowing what it is.

There is no colour, anywhere. No `await`, no async member, and no second spelling of any `Core` member
that touches I/O. The suspension route travels in the context rather than in the type, which is
precisely what makes that possible.

What it costs is that a stack is the unit of concurrency
(`rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`): a task is about a megabyte of address
space rather than the hundred bytes a stackless future would be. Simplicity of the language surface
outranks footprint, and that trade is taken knowingly.
