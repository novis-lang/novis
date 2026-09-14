<?php
// The file that must leave this extension asleep. Novis parses PHP, and the extension still claims `.nvs`
// alone (`rule:ide/the-extension-claims-nvs-only`), so opening this one activates nothing of ours and the
// PHP extension a user already has keeps the file.

function total(array $items): int
{
    $sum = 0;
    foreach ($items as $item) {
        $sum += $item;
    }
    return $sum;
}

echo total([2, 3, 5]), "\n";
