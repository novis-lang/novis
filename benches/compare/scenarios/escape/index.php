<?php
// Prints the search term `?q=` into a heading with the short echo tag on `/`,
// and with `echo` on `/direct`.
$term = $_GET['q'] ?? '';
if (parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH) === '/direct') {
    echo "<h1>Results for $term</h1>\n";
    exit;
}
?>
<h1>Results for <?= $term ?></h1>
