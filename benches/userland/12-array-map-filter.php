<?php
// map / filter / reduce over a list - the pipeline that replaced the foreach.
final class Bench {
    public static function run(int $size, int $rounds): int {
        $source = [];
        $i = 0;
        while ($i < $size) {
            $source[] = $i;
            $i = $i + 1;
        }

        $total = 1;
        $round = 0;
        while ($round < $rounds) {
            $source[$round % $size] = $total % 1000;
            $doubled = array_map(fn(int $n): int => $n * 2, $source);
            $kept = array_filter($doubled, fn(int $n): bool => $n % 3 == 0);
            $total = $total + array_reduce($kept, fn(int $carry, int $n): int => $carry + $n, 0);
            $round = $round + 1;
        }
        return $total;
    }
}

echo Bench::run(100000, 20), "\n";
