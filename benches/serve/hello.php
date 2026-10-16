<?php
// The server case: what one request costs before any application work does.
//
// The twin of `hello.nvs`, byte for byte on stdout. Run behind `php-cgi -b`
// (the FastCGI SAPI, which is what FPM is) or `php -S`, both with opcache on;
// `tools/nv/cmd/bench.ts`'s `# The serve-versus-FPM leg` says which and why.
echo "hello, world\n";
