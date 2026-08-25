<?php
// A hot integer loop: the arithmetic every counter, accumulator and offset walk is made of.
final class Bench {
    public static function run(int $rounds): int {
        $total = 0;
        $i = 0;
        while ($i < $rounds) {
            $total = ($total + $i * 3 - 1) % 1000003;
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(3000000), "\n";
