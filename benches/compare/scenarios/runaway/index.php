<?php
// Loops forever for `/loop`, and adds 1 MiB to an array forever for `/alloc`.
// Every other path prints `ok`.
$path = parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH);
if ($path === '/loop') {
    $turns = 0;
    while (true) {
        $turns++;
    }
}
if ($path === '/alloc') {
    $chunks = [];
    while (true) {
        $chunks[] = str_repeat('x', 1048576);
    }
}
echo "ok\n";
