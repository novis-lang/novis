<?php
// One visitor's session, read and written by two kinds of request.
// `/slow` adds 1 to a counter in the session, then works for 2 seconds.
// `/fast` only reads the counter. Every other path starts the session.

session_start();
$path = parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH);
header('Content-Type: text/plain; charset=UTF-8');

if ($path === '/slow') {
    $_SESSION['count'] = ($_SESSION['count'] ?? 0) + 1;
    sleep(2);
    echo "slow done\n";
} elseif ($path === '/fast') {
    echo 'count: ', $_SESSION['count'] ?? 0, "\n";
} else {
    $_SESSION['count'] = 0;
    echo "started\n";
}
