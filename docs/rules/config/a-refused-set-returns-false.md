A `Core\Config::set` that the changeability class or a ceiling refuses **returns `false` and leaves
the value in force untouched**. It is never clamped to the ceiling: silently running with a different
number than the one requested is harder to diagnose than a false return.

Every refusal answers the same way — a `System` directive whatever the value, a `RuntimeTighten`
widening, a `Runtime` value above `[limits.hard]`, a value that does not spell the unit its
directive takes, a name no directive governs — and none of them throws, so a program cannot catch a
refusal and cannot tell one reason from another by its answer. What it can rely on is that after a
`false` nothing about the request's configuration moved.

The accepted half is the other assertion: a set below the ceiling does not merely echo back through
`get`, it **takes effect** — a raised `memory` moves the ceiling the runtime enforces for the rest of
that request (`rule:errors/on-limit`).
