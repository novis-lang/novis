- **An option of a trailing bag is not a named argument at the call site, and writing one is `E0486`
  listing parameters the member never documented.** `$store->getSecret("k", $ring, fill: $f)` is
  refused with "the parameters are: `key:`, `keys:`, `options:`", because the bag is one parameter
  and `rule:core-api/shape-rules` R2's "callable by name" is about the keys *inside* it. Write the
  bag out — `$store->getSecret("k", $ring, {fill: $f})` — which is the spelling every `put`-with-a
  -`ttl` case already uses. [until: gone crates/nvs-diagnostics/src/lib.rs:E0486]
