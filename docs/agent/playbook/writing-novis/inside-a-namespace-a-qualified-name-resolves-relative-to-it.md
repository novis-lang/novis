- **Inside a `namespace`, a qualified name resolves relative to it — `Core\` included.** After
  `namespace App;`, `Core\Str::length("abc")` and `#[Core\Route(...)]` are `E0303: App\Core\Str is
  not declared`, which names the joined path rather than the missing backslash. Write `use
  Core\Str;` or the leading `\` — the same rule makes `App\User::class` inside `namespace App;`
  `App\App\User`, as PHP does. [until: reviewed 2026-09-06]
