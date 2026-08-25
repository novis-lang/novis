<?php
// preg_match with capture groups - how userland parses a log line, a header or an id.
final class Bench {
    public static function run(int $rounds): int {
        $lines = [];
        $k = 0;
        while ($k < 16) {
            $lines[] = "2026-08-25 12:04:31 " . str_repeat("A", $k + 1) . " /orders/4711 200 138ms";
            $k = $k + 1;
        }

        $total = 1;
        $i = 0;
        while ($i < $rounds) {
            $found = [];
            if (preg_match('/(\w+) \/orders\/(\d+) (\d{3}) (\d+)ms/', $lines[$total % 16], $found) === 1) {
                $total = $total + strlen($found[1]);
            }
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(200000), "\n";
