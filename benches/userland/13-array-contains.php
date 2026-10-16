<?php
// in_array over a list — the membership test userland reaches for before it reaches for a set.
final class Bench {
    public static function run(int $size, int $probes): int {
        $allowed = [];
        $i = 0;
        while ($i < $size) {
            $allowed[] = "role-" . $i;
            $i = $i + 1;
        }

        $hits = 0;
        $probe = 0;
        while ($probe < $probes) {
            if (in_array("role-" . ($probe * 7 % ($size * 2)), $allowed, true)) {
                $hits = $hits + 1;
            }
            $probe = $probe + 1;
        }
        return $hits;
    }
}

echo Bench::run(500, 20000), "\n";
