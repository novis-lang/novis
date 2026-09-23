- **`php` inside WSL says whether a float landmark is exact on both legs, without a Linux build of
  `nvs`.** PHP calls the same libm Rust's `f64` methods do, so `wsl.exe -- bash -lc "php
  /mnt/<drive>/<repo>/.agent-tmp/rows.php"` answers whether glibc rounds it the way MSVC does.
  Domain endpoints and halvings (`acos(-1.0) == PI`, `tanh(20.0) == 1.0`) are safe equalities on
  both legs; the interior (`sin($x) * sin($x) + cos($x) * cos($x)`) needs the tolerance spelling.
  [until: reviewed 2026-09-06]
