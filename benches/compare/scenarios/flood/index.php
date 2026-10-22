<?php
// `/hold` keeps 8 MiB for 300 ms and then prints `held`.
// Every other path prints `ok`.
$path = parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH);
if ($path === '/hold') {
    $held = str_repeat('x', 8388608);
    usleep(300000);
    if (str_ends_with($held, 'x')) {
        echo "held\n";
    }
} else {
    echo "ok\n";
}
