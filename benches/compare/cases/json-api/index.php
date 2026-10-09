<?php
// Reads a JSON order, checks it, and returns its totals in cents as JSON.
// This measures reading, checking and writing JSON in a typical API request.

// Returns the receipt for one decoded order, or null when the order fails a check.
function total(mixed $order): ?array {
    if (!is_array($order) || !is_array($order['customer'] ?? null) || !is_array($order['items'] ?? null)) {
        return null;
    }
    $customer = $order['customer'];
    if (!is_int($customer['id'] ?? null) || !is_string($customer['name'] ?? null) || !is_string($customer['email'] ?? null)) {
        return null;
    }
    $items = $order['items'];
    if ($customer['id'] < 1 || !array_is_list($items) || count($items) < 1 || count($items) > 100) {
        return null;
    }
    $lines = [];
    $subtotal = 0;
    foreach ($items as $item) {
        if (!is_array($item) || !is_string($item['sku'] ?? null) || !is_string($item['name'] ?? null)
            || !is_int($item['qty'] ?? null) || !is_int($item['unitCents'] ?? null)) {
            return null;
        }
        if ($item['sku'] === '' || $item['qty'] < 1 || $item['qty'] > 1000 || $item['unitCents'] < 1 || $item['unitCents'] > 10000000) {
            return null;
        }
        $total = $item['qty'] * $item['unitCents'];
        $lines[] = ['sku' => $item['sku'], 'qty' => $item['qty'], 'totalCents' => $total];
        $subtotal += $total;
    }

    // The tax is 20 percent, rounded to the nearest cent. Half a cent rounds up.
    $tax = intdiv($subtotal * 20 + 50, 100);

    // The largest line comes first. `usort` is stable, so lines with the same total keep the order of the request.
    usort($lines, fn(array $a, array $b): int => $b['totalCents'] <=> $a['totalCents']);

    return ['customer' => $customer['id'], 'items' => $lines, 'subtotalCents' => $subtotal, 'taxCents' => $tax, 'totalCents' => $subtotal + $tax];
}

header('Content-Type: application/json');
if ($_SERVER['REQUEST_METHOD'] !== 'POST' || parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH) !== '/orders') {
    http_response_code(404);
    header('Content-Type: text/plain; charset=UTF-8');
    echo "not found\n";
    return;
}

$receipt = total(json_decode(file_get_contents('php://input'), true));
if ($receipt === null) {
    http_response_code(422);
    echo json_encode(['error' => 'invalid order']);
} else {
    echo json_encode($receipt, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
}
