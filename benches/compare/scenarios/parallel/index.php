<?php
// Runs `?parts=` parts of the same loop and prints the sum of their results.
// PHP-FPM runs one request on one core, so the parts run one after another.
function part(int $seed, int $turns): int
{
    $x = $seed;
    for ($i = 0; $i < $turns; $i++) {
        $x = ($x * 31 + $i) % 1000003;
    }
    return $x;
}

$parts = (int) ($_GET['parts'] ?? 1);
$turns = (int) ($_GET['turns'] ?? 0);
$sum = 0;
for ($k = 1; $k <= $parts; $k++) {
    $sum += part($k, $turns);
}
echo $sum;
