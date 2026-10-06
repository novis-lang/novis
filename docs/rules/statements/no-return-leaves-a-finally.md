`return` inside `finally` does not compile. It would replace whatever the region was leaving with —
including a throw in flight, which would be discarded with no `catch` anywhere in the program, the
one way an exception could vanish without a handler. The diagnostic names the two rewrites: change
the result in a `catch`, or after the region.

`break` and `continue` whose target lies outside the `finally` are refused on the same grounds; a
loop wholly inside the block keeps both.

Whether an override was a bug (usually) or intent is the author's call, so the diagnostic names both
rewrites and picks neither.
