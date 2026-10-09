<?php
// Waits 50 ms on a timer, then answers. This measures the cost of many
// requests in flight at once, as when a page waits for another service.

header('Content-Type: text/plain; charset=UTF-8');
if (parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH) !== '/wait') {
    http_response_code(404);
    echo "not found\n";
    return;
}

// `usleep` holds this FPM worker for the whole wait, so the pool needs one worker per request in flight.
usleep(50000);
echo "waited 50 ms\n";
