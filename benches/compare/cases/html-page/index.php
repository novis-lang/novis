<?php
// Shows a table of 100 products, with the search term in the heading.
// This measures building an HTML page and escaping text into it.

if (parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH) !== '/products') {
    http_response_code(404);
    header('Content-Type: text/plain; charset=UTF-8');
    echo "not found\n";
    return;
}

$term = $_GET['q'] ?? '';
if (!is_string($term)) {
    $term = '';
}

// The products are built again for every request, as a page reads them from a database.
$products = [];
for ($i = 1; $i <= 100; $i++) {
    $products[] = ['id' => $i, 'name' => sprintf('Teapot %d & lid', $i), 'priceCents' => 100 + ($i * 7919) % 90000];
}

// `htmlspecialchars` escapes `&`, `<`, `>`, `"` and `'` in the term and in every name.
$rows = '';
foreach ($products as $product) {
    $price = sprintf('%d.%02d', intdiv($product['priceCents'], 100), $product['priceCents'] % 100);
    $rows .= '<tr><td>' . $product['id'] . '</td><td>' . htmlspecialchars($product['name']) . '</td><td>'
        . htmlspecialchars($price) . "</td></tr>\n";
}

echo "<!DOCTYPE html>\n"
    . "<html><head><meta charset=\"utf-8\"><title>Products</title></head><body>\n"
    . '<h1>Results for ' . htmlspecialchars($term) . "</h1>\n"
    . "<table>\n"
    . "<tr><th>Id</th><th>Name</th><th>Price</th></tr>\n"
    . $rows
    . "</table>\n"
    . "</body></html>\n";
