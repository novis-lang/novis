<?php
// Prints the search term `?q=` into a heading with the short echo tag.
$term = $_GET['q'] ?? '';
?>
<h1>Results for <?= $term ?></h1>
