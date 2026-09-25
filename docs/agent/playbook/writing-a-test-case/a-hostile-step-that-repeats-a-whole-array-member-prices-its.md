- **A hostile step that repeats a whole-array member prices its repeat count by the entries, and a
  count copied from a sibling attack is then wrong by orders of magnitude.**
  `Core\Arr::chunk($long, 25000)` over 100000 entries, 20000 times, is two billion entry copies: it
  ran 3m23s in a debug build against a declared `timeout-ms 60000`, where the same 20000 repeats of
  `Core\Arr::firstKey` cost nothing because that member reads one end. Nothing grows here, so this is
  not the quadratic-append trap above; divide the repeat count by the entries the member walks, and
  time the file yourself before committing it. [until: gone crates/nvs-stdlib/src/arr.rs:Core\Arr::firstKey]
