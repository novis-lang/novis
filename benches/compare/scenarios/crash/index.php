<?php
// Throws an error for the path `/crash`, and prints `ok` for every other path.
if (parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH) === '/crash') {
    throw new RuntimeException('the handler failed');
}
echo "ok\n";
