- **Indexing an `array<mixed>` gives `mixed`, so a case that walks a nested structure needs
  `as array<mixed>` on every step down.** `$node = $node["next"];` is `E0401` — `mixed` is not
  `array<mixed>` — which makes a depth-counting assertion look impossible to write from Novis when
  it is one cast away. Write `$node = $node["next"] as array<mixed>;`, and reach for it whenever a
  case descends what it built with `$node = ["next" => $node];`. [until: gone crates/nvs-diagnostics/src/lib.rs:E_SUBSCRIPT_ON_NON_ARRAY]
