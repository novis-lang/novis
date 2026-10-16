<?php
// Sorting a list of numbers, from the same seeded input on both sides.
final class Bench {
    public static function run(int $size, int $rounds): int {
        $source = [];
        $seed = 12345;
        $i = 0;
        while ($i < $size) {
            $seed = ($seed * 1103515245 + 12345) % 2147483648;
            $source[] = $seed % 100000;
            $i = $i + 1;
        }

        $total = 1;
        $round = 0;
        while ($round < $rounds) {
            $source[$round % $size] = $total % 100000;
            $sorted = $source;
            sort($sorted);
            $total = $total + $sorted[0] + $sorted[count($sorted) - 1];
            $round = $round + 1;
        }
        return $total;
    }
}

echo Bench::run(50000, 20), "\n";
