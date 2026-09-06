`return` inside `finally` does not compile. It would replace whatever the region was leaving with —
including a throw in flight, which would be discarded with no `catch` anywhere in the program — and
it is the only construct in PHP where an unhandled exception vanishes without a handler. PHP 8.6
deprecates the spelling for removal on exactly that argument. The diagnostic names the two rewrites:
change the result in a `catch`, or after the region.

`break` and `continue` whose target lies outside the `finally` are refused on the same grounds; a
loop wholly inside the block keeps both.

This is the one PHP 8.6 refusal with no mechanical rewrite. Whether the override was a bug (usually)
or intent is a human's call, so `nvs convert` points at the line and rewrites nothing.
