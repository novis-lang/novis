<?php
// Growing a list one element at a time, then walking it - the result set of every query loop.
final class Bench {
    public static function run(int $rows): int {
        $items = [];
        $seed = 1;
        $i = 0;
        while ($i < $rows) {
            $seed = ($seed * 31 + $i) % 9973;
            $items[] = $seed;
            $i = $i + 1;
        }

        $total = 0;
        foreach ($items as $value) {
            $total = $total + $value;
        }
        return $total + count($items);
    }
}

echo Bench::run(1000000), "\n";
