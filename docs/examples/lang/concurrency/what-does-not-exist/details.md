`Thread`, `Promise` and `parallel\Runtime` are not classes. `new Thread()` is `E0303`, and the
message names no replacement.

`pcntl_fork()` and `curl_multi_init()` are `E0320`, because Novis has no functions outside a class.

`spawn worker "x.nvs"` is a general syntax error. The first error is `E0319` for `spawn`, and it names
no replacement.

`global $x;` is `E0204`, `$GLOBALS` is `E0211`, and `static int $n = 0;` inside a method is `E0209`.
Their help names a `static` property, a constant or a parameter.
