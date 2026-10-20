<?php
// The server case: what one request costs before any application work does.
//
// The twin of `hello.nvs`, byte for byte on stdout. `bun nv bench-proxied`
// serves it from PHP-FPM with opcache on; `benches/proxied/README.md` says why.
echo "hello, world\n";
