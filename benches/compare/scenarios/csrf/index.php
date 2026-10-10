<?php
// A `POST` to `/change` changes the account and prints `changed`.
if ($_SERVER['REQUEST_METHOD'] === 'POST' && parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH) === '/change') {
    echo "changed\n";
} else {
    http_response_code(404);
}
